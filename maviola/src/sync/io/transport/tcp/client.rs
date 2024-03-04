use std::net::TcpStream;
use std::thread;
use std::thread::JoinHandle;

use crate::core::io::ChannelInfo;
use crate::core::utils::Closer;
use crate::sync::consts::CONN_STOP_POOLING_INTERVAL;
use crate::sync::io::{Connection, ConnectionBuilder};

use crate::prelude::*;

impl<V: MaybeVersioned + 'static> ConnectionBuilder<V> for TcpClient {
    fn build(&self) -> Result<(Connection<V>, JoinHandle<Result<Closer>>)> {
        let server_addr = self.addr;
        let writer = TcpStream::connect(server_addr)?;
        let reader = writer.try_clone()?;

        let conn_state = Closer::new();
        let (connection, chan_factory) = Connection::new(self.info.clone(), conn_state.to_shared());

        let channel = chan_factory.build(ChannelInfo::TcpClient { server_addr }, reader, writer);
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
