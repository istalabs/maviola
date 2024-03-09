use async_trait::async_trait;
use tokio::fs::File;
use tokio::io::BufWriter;

use crate::asnc::io::{Connection, ConnectionBuilder, ConnectionHandler};
use crate::asnc::marker::AsyncConnConf;
use crate::asnc::utils::BusyReader;
use crate::core::io::ChannelInfo;
use crate::core::utils::SharedCloser;

use crate::prelude::*;

#[async_trait]
impl<V: MaybeVersioned + 'static> ConnectionBuilder<V> for FileWriter {
    async fn build(&self) -> Result<(Connection<V>, ConnectionHandler)> {
        let path = self.path.clone();
        let file = File::create(path.as_path()).await?;

        let writer = BufWriter::new(file);
        let reader = BusyReader;

        let (connection, chan_factory) = Connection::new(self.info.clone(), SharedCloser::new());

        let channel = chan_factory.build(ChannelInfo::FileWriter { path }, reader, writer);
        let channel_state = channel.spawn().await;

        let handler = ConnectionHandler::spawn_from_state(channel_state);

        Ok((connection, handler))
    }

    fn to_conf(&self) -> AsyncConnConf<V> {
        AsyncConnConf::new(self.clone())
    }
}
