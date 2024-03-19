use std::fs::File;
use std::io::BufWriter;

use crate::core::io::ChannelDetails;
use crate::core::utils::SharedCloser;
use crate::sync::io::{Connection, ConnectionBuilder, ConnectionHandler};
use crate::sync::utils::BusyReader;

use crate::prelude::*;
use crate::sync::marker::ConnConf;

impl<V: MaybeVersioned> ConnectionBuilder<V> for FileWriter {
    fn build(&self) -> Result<(Connection<V>, ConnectionHandler)> {
        let path = self.path.clone();
        let file = File::create(path.as_path())?;

        let writer = BufWriter::new(file);
        let reader = BusyReader;

        let (connection, chan_factory) = Connection::new(self.info.clone(), SharedCloser::new());

        let chan_info = connection
            .info()
            .make_channel_info(ChannelDetails::FileWriter { path });
        let channel = chan_factory.build(chan_info, reader, writer);
        let channel_state = channel.spawn();

        let handler = ConnectionHandler::spawn_from_state(channel_state);

        Ok((connection, handler))
    }

    fn to_conf(&self) -> ConnConf<V> {
        ConnConf::new(self.clone())
    }
}
