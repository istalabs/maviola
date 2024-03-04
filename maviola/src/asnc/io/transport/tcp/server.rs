use async_trait::async_trait;
use tokio::net::TcpListener;

use crate::asnc::io::{Connection, ConnectionBuilder};
use crate::asnc::utils::handle_listener_stop;
use crate::core::io::ChannelInfo;
use crate::core::utils::Closer;

use crate::prelude::*;

#[async_trait]
impl<V: MaybeVersioned + 'static> ConnectionBuilder<V> for TcpServer {
    async fn build(&self) -> Result<Connection<V>> {
        let server_addr = self.addr;
        let listener = TcpListener::bind(self.addr).await?;

        let conn_state = Closer::new();
        let (connection, chan_factory) = Connection::new(self.info.clone(), conn_state.to_shared());

        let handler: tokio::task::JoinHandle<Result<Closer>> = tokio::spawn(async move {
            loop {
                if conn_state.is_closed() {
                    return Ok(conn_state);
                }

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
        });

        handle_listener_stop(handler, connection.info().clone());

        Ok(connection)
    }
}
