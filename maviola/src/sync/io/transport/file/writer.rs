use std::fs::File;
use std::io::BufWriter;

use crate::core::io::ChannelInfo;
use crate::core::utils::SharedCloser;
use crate::sync::io::{Connection, ConnectionBuilder};
use crate::sync::utils::BusyReader;

use crate::prelude::*;

impl<V: MaybeVersioned + 'static> ConnectionBuilder<V> for FileWriter {
    fn build(&self) -> Result<Connection<V>> {
        let path = self.path.clone();
        let file = File::create(path.as_path())?;

        let writer = BufWriter::new(file);
        let reader = BusyReader;

        let conn_state = SharedCloser::new();
        let (connection, peer_builder) = Connection::new(self.info.clone(), conn_state);

        let peer_connection = peer_builder.build(ChannelInfo::FileWriter { path }, reader, writer);
        peer_connection.spawn().discard();

        Ok(connection)
    }
}
