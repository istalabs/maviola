use std::net::UdpSocket;
use std::thread;
use std::thread::JoinHandle;

use crate::core::io::ChannelInfo;
use crate::core::utils::net::{pick_unused_port, resolve_socket_addr};
use crate::core::utils::Closer;
use crate::sync::io::transport::udp::udp_rw::UdpRW;
use crate::sync::io::{Connection, ConnectionBuilder};

use crate::prelude::*;
use crate::sync::consts::CONN_STOP_POOLING_INTERVAL;

impl<V: MaybeVersioned + 'static> ConnectionBuilder<V> for UdpClient {
    fn build(&self) -> Result<(Connection<V>, JoinHandle<Result<Closer>>)> {
        let bind_addr = match self.bind_addr {
            None => resolve_socket_addr(format!("{}:{}", self.host, pick_unused_port()?))?,
            Some(bind_addr) => bind_addr,
        };
        let server_addr = self.addr;

        let udp_socket = UdpSocket::bind(bind_addr)?;
        udp_socket.connect(server_addr)?;

        let writer = UdpRW::new(udp_socket);
        let reader = writer.try_clone()?;

        let conn_state = Closer::new();
        let (connection, chan_factory) = Connection::new(self.info.clone(), conn_state.to_shared());

        let channel = chan_factory.build(
            ChannelInfo::UdpClient {
                server_addr,
                bind_addr,
            },
            reader,
            writer,
        );
        let channel_state = channel.spawn();

        let handler = thread::spawn(move || -> Result<Closer> {
            while !channel_state.is_closed() {
                thread::sleep(CONN_STOP_POOLING_INTERVAL);
            }
            Ok(conn_state)
        });

        Ok((connection, handler))
    }
}
