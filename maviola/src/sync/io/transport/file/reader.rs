use std::fs::File;
use std::io::BufReader;

use crate::core::io::ChannelInfo;
use crate::core::utils::SharedCloser;
use crate::sync::io::{Connection, ConnectionBuilder};
use crate::sync::utils::BusyWriter;

use crate::prelude::*;

impl<V: MaybeVersioned + 'static> ConnectionBuilder<V> for FileReader {
    fn build(&self) -> Result<Connection<V>> {
        let path = self.path.clone();
        let file = File::open(path.as_path())?;

        let writer = BusyWriter;
        let reader = BufReader::new(file);

        let conn_state = SharedCloser::new();
        let (connection, peer_builder) = Connection::new(self.info.clone(), conn_state);

        let peer_connection = peer_builder.build(ChannelInfo::FileReader { path }, reader, writer);
        peer_connection.spawn().discard();

        Ok(connection)
    }
}
