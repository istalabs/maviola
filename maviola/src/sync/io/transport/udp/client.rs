use std::net::UdpSocket;

use crate::core::io::ChannelInfo;
use crate::core::utils::net::{pick_unused_port, resolve_socket_addr};
use crate::core::utils::SharedCloser;
use crate::sync::io::transport::udp::udp_rw::UdpRW;
use crate::sync::io::{Connection, ConnectionBuilder, ConnectionHandler};
use crate::sync::marker::ConnConf;

use crate::prelude::*;

impl<V: MaybeVersioned + 'static> ConnectionBuilder<V> for UdpClient {
    fn build(&self) -> Result<(Connection<V>, ConnectionHandler)> {
        let bind_addr = match self.bind_addr {
            None => resolve_socket_addr(format!("{}:{}", self.host, pick_unused_port()?))?,
            Some(bind_addr) => bind_addr,
        };
        let server_addr = self.addr;

        let udp_socket = UdpSocket::bind(bind_addr)?;
        udp_socket.connect(server_addr)?;

        let writer = UdpRW::new(udp_socket);
        let reader = writer.try_clone()?;

        let (connection, chan_factory) = Connection::new(self.info.clone(), SharedCloser::new());

        let channel = chan_factory.build(
            ChannelInfo::UdpClient {
                server_addr,
                bind_addr,
            },
            reader,
            writer,
        );
        let channel_state = channel.spawn();

        let handler = ConnectionHandler::spawn_from_state(channel_state);

        Ok((connection, handler))
    }

    fn to_conf(&self) -> ConnConf<V> {
        ConnConf::new(self.clone())
    }

    fn is_repairable(&self) -> bool {
        true
    }
}
