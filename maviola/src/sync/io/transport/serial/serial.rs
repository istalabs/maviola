use serialport;

use crate::core::io::ChannelDetails;
use crate::core::utils::SharedCloser;
use crate::sync::io::{Connection, ConnectionBuilder, ConnectionHandler};
use crate::sync::marker::ConnConf;

use crate::prelude::*;

impl<V: MaybeVersioned> ConnectionBuilder<V> for SerialPort {
    fn build(&self) -> Result<(Connection<V>, ConnectionHandler)> {
        let path = self.path.clone();
        let baud_rate = self.baud_rate;

        let writer = serialport::new(&self.path, self.baud_rate)
            .timeout(self.conn_timeout)
            .open()?;
        let reader = writer.try_clone()?;

        let (connection, chan_factory) = Connection::new(self.info.clone(), SharedCloser::new());

        let chan_info = connection
            .info()
            .make_channel_info(ChannelDetails::SerialPort { path, baud_rate });
        let channel = chan_factory.build(chan_info, reader, writer);
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
