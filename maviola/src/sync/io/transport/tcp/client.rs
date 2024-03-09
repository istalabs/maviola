use std::net::TcpStream;

use crate::core::io::ChannelInfo;
use crate::core::utils::SharedCloser;
use crate::sync::io::{Connection, ConnectionBuilder, ConnectionHandler};
use crate::sync::marker::ConnConf;

use crate::prelude::*;

impl<V: MaybeVersioned + 'static> ConnectionBuilder<V> for TcpClient {
    fn build(&self) -> Result<(Connection<V>, ConnectionHandler)> {
        let server_addr = self.addr;
        let writer = TcpStream::connect(server_addr)?;
        let reader = writer.try_clone()?;

        let (connection, chan_factory) = Connection::new(self.info.clone(), SharedCloser::new());

        let channel = chan_factory.build(ChannelInfo::TcpClient { server_addr }, reader, writer);
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
