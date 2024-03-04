use std::net::TcpStream;

use crate::core::io::ChannelInfo;
use crate::core::utils::SharedCloser;
use crate::sync::io::{Connection, ConnectionBuilder};

use crate::prelude::*;

impl<V: MaybeVersioned + 'static> ConnectionBuilder<V> for TcpClient {
    fn build(&self) -> Result<Connection<V>> {
        let server_addr = self.addr;
        let writer = TcpStream::connect(server_addr)?;
        let reader = writer.try_clone()?;

        let conn_state = SharedCloser::new();
        let (connection, peer_builder) = Connection::new(self.info.clone(), conn_state);

        let peer_connection =
            peer_builder.build(ChannelInfo::TcpClient { server_addr }, reader, writer);
        peer_connection.spawn().discard();

        Ok(connection)
    }
}
