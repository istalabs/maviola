use std::os::unix::net::UnixStream;

use crate::core::io::ChannelInfo;
use crate::core::utils::SharedCloser;
use crate::sync::io::{Connection, ConnectionBuilder};

use crate::prelude::*;

impl<V: MaybeVersioned + 'static> ConnectionBuilder<V> for SockClient {
    fn build(&self) -> Result<Connection<V>> {
        let path = self.path.clone();
        let writer = UnixStream::connect(path.as_path())?;
        let reader = writer.try_clone()?;

        let conn_state = SharedCloser::new();
        let (connection, peer_builder) = Connection::new(self.info.clone(), conn_state);

        let peer_connection = peer_builder.build(ChannelInfo::SockClient { path }, reader, writer);
        peer_connection.spawn().discard();

        Ok(connection)
    }
}
