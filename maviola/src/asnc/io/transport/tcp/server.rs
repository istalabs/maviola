use async_trait::async_trait;
use tokio::net::TcpListener;

use crate::asnc::io::{Connection, ConnectionBuilder, ConnectionHandler};
use crate::core::io::ChannelInfo;
use crate::core::utils::Closer;

use crate::prelude::*;

#[async_trait]
impl<V: MaybeVersioned + 'static> ConnectionBuilder<V> for TcpServer {
    async fn build(&self) -> Result<(Connection<V>, ConnectionHandler)> {
        let server_addr = self.addr;
        let listener = TcpListener::bind(self.addr).await?;

        let conn_state = Closer::new();
        let (connection, chan_factory) = Connection::new(self.info.clone(), conn_state.to_shared());

        let handler = ConnectionHandler::spawn(async move {
            while !conn_state.is_closed() {
                let (stream, peer_addr) = listener.accept().await?;

                let (reader, writer) = stream.into_split();

                let channel = chan_factory.build(
                    ChannelInfo::TcpServer {
                        server_addr,
                        peer_addr,
                    },
                    reader,
                    writer,
                );
                channel.spawn().await.discard();
            }

            Ok(())
        });

        Ok((connection, handler))
    }
}
