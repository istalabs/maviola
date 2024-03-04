use std::fs::File;
use std::io::BufWriter;
use std::thread;
use std::thread::JoinHandle;

use crate::core::io::ChannelInfo;
use crate::core::utils::Closer;
use crate::sync::consts::CONN_STOP_POOLING_INTERVAL;
use crate::sync::io::{Connection, ConnectionBuilder};
use crate::sync::utils::BusyReader;

use crate::prelude::*;

impl<V: MaybeVersioned + 'static> ConnectionBuilder<V> for FileWriter {
    fn build(&self) -> Result<(Connection<V>, JoinHandle<Result<Closer>>)> {
        let path = self.path.clone();
        let file = File::create(path.as_path())?;

        let writer = BufWriter::new(file);
        let reader = BusyReader;

        let mut conn_state = Closer::new();
        let (connection, chan_factory) = Connection::new(self.info.clone(), conn_state.to_shared());

        let channel = chan_factory.build(ChannelInfo::FileWriter { path }, reader, writer);
        let channel_state = channel.spawn();

        let handler = thread::spawn(move || -> Result<Closer> {
            while !channel_state.is_closed() {
                thread::sleep(CONN_STOP_POOLING_INTERVAL);
            }
            conn_state.close();
            Ok(conn_state)
        });

        Ok((connection, handler))
    }
}
