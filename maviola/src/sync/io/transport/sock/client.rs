use std::os::unix::net::UnixStream;
use std::thread;
use std::thread::JoinHandle;

use crate::core::io::ChannelInfo;
use crate::core::utils::Closer;
use crate::sync::consts::CONN_STOP_POOLING_INTERVAL;
use crate::sync::io::{Connection, ConnectionBuilder};

use crate::prelude::*;

impl<V: MaybeVersioned + 'static> ConnectionBuilder<V> for SockClient {
    fn build(&self) -> Result<(Connection<V>, JoinHandle<Result<Closer>>)> {
        let path = self.path.clone();
        let writer = UnixStream::connect(path.as_path())?;
        let reader = writer.try_clone()?;

        let conn_state = Closer::new();
        let (connection, chan_factory) = Connection::new(self.info.clone(), conn_state.to_shared());

        let channel = chan_factory.build(ChannelInfo::SockClient { path }, reader, writer);
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
