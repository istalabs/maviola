use async_trait::async_trait;
use tokio::net::TcpStream;

use crate::asnc::io::{Connection, ConnectionBuilder};
use crate::core::io::ChannelInfo;
use crate::core::utils::SharedCloser;

use crate::prelude::*;

#[async_trait]
impl<V: MaybeVersioned + 'static> ConnectionBuilder<V> for TcpClient {
    async fn build(&self) -> Result<Connection<V>> {
        let server_addr = self.addr;
        let stream = TcpStream::connect(server_addr).await?;
        let (reader, writer) = stream.into_split();

        let conn_state = SharedCloser::new();
        let (connection, chan_factory) = Connection::new(self.info.clone(), conn_state);

        let channel = chan_factory.build(ChannelInfo::TcpClient { server_addr }, reader, writer);
        channel.spawn().await.discard();

        Ok(connection)
    }
}
