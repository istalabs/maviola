use async_trait::async_trait;
use tokio_serial::SerialPortBuilderExt;

use crate::asnc::io::transport::serial::serial_rw::SerialRW;
use crate::asnc::io::{Connection, ConnectionBuilder, ConnectionHandler};
use crate::asnc::marker::AsyncConnConf;
use crate::core::io::ChannelDetails;
use crate::core::utils::SharedCloser;

use crate::prelude::*;

#[async_trait]
impl<V: MaybeVersioned> ConnectionBuilder<V> for SerialPort {
    async fn build(&self) -> Result<(Connection<V>, ConnectionHandler)> {
        let path = self.path.clone();
        let baud_rate = self.baud_rate;

        let serial_port = tokio_serial::new(&path, baud_rate)
            .timeout(self.conn_timeout)
            .open_native_async()?;

        let writer = SerialRW::new(serial_port);
        let reader = writer.clone();

        let (connection, chan_factory) = Connection::new(self.info.clone(), SharedCloser::new());

        let chan_info = connection
            .info()
            .make_channel_info(ChannelDetails::SerialPort { path, baud_rate });
        let channel = chan_factory.build(chan_info, reader, writer);
        let channel_state = channel.spawn().await;

        let handler = ConnectionHandler::spawn_from_state(channel_state);

        Ok((connection, handler))
    }

    fn to_conf(&self) -> AsyncConnConf<V> {
        AsyncConnConf::new(self.clone())
    }

    fn is_repairable(&self) -> bool {
        true
    }
}
