use async_trait::async_trait;
use tokio::net::TcpStream;

use crate::asnc::io::{Connection, ConnectionBuilder, ConnectionHandler};
use crate::asnc::marker::AsyncConnConf;
use crate::core::io::ChannelInfo;
use crate::core::utils::SharedCloser;

use crate::prelude::*;

#[async_trait]
impl<V: MaybeVersioned + 'static> ConnectionBuilder<V> for TcpClient {
    async fn build(&self) -> Result<(Connection<V>, ConnectionHandler)> {
        let server_addr = self.addr;
        let stream = TcpStream::connect(server_addr).await?;
        let (reader, writer) = stream.into_split();

        let (connection, chan_factory) = Connection::new(self.info.clone(), SharedCloser::new());

        let channel = chan_factory.build(ChannelInfo::TcpClient { server_addr }, reader, writer);
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
