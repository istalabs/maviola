use async_trait::async_trait;
use tokio::fs::File;
use tokio::io::BufReader;

use crate::asnc::io::{Connection, ConnectionBuilder, ConnectionHandler};
use crate::asnc::marker::AsyncConnConf;
use crate::asnc::utils::BusyWriter;
use crate::core::io::ChannelDetails;
use crate::core::utils::SharedCloser;

use crate::prelude::*;

#[async_trait]
impl<V: MaybeVersioned> ConnectionBuilder<V> for FileReader {
    async fn build(&self) -> Result<(Connection<V>, ConnectionHandler)> {
        let path = self.path.clone();
        let file = File::open(path.as_path()).await?;

        let writer = BusyWriter;
        let reader = BufReader::new(file);

        let (connection, chan_factory) = Connection::new(self.info.clone(), SharedCloser::new());

        let chan_info = connection
            .info()
            .make_channel_info(ChannelDetails::FileReader { path });
        let channel = chan_factory.build(chan_info, reader, writer);
        let channel_state = channel.spawn().await;

        let handler = ConnectionHandler::spawn_from_state(channel_state);

        Ok((connection, handler))
    }

    fn to_conf(&self) -> AsyncConnConf<V> {
        AsyncConnConf::new(self.clone())
    }
}
