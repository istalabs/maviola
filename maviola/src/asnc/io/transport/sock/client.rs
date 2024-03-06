use async_trait::async_trait;
use tokio::net::UnixStream;

use crate::asnc::io::{Connection, ConnectionBuilder, ConnectionHandler};
use crate::core::io::ChannelInfo;
use crate::core::utils::SharedCloser;

use crate::prelude::*;

#[async_trait]
impl<V: MaybeVersioned + 'static> ConnectionBuilder<V> for SockClient {
    async fn build(&self) -> Result<(Connection<V>, ConnectionHandler)> {
        let path = self.path.clone();
        let stream = UnixStream::connect(path.as_path()).await?;
        let (reader, writer) = stream.into_split();

        let (connection, chan_factory) = Connection::new(self.info.clone(), SharedCloser::new());

        let channel = chan_factory.build(ChannelInfo::SockClient { path }, reader, writer);
        let channel_state = channel.spawn().await;

        let handler = ConnectionHandler::spawn_from_state(channel_state);

        Ok((connection, handler))
    }
}