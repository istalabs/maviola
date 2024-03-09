use std::net::SocketAddr;

use async_trait::async_trait;
use tokio::net::{TcpListener, TcpStream};

use crate::asnc::io::{Connection, ConnectionBuilder, ConnectionHandler};
use crate::asnc::marker::AsyncConnConf;
use crate::core::consts::SERVER_HANG_UP_TIMEOUT;
use crate::core::io::ChannelInfo;
use crate::core::utils::{Closable, Closer};

use crate::prelude::*;

#[async_trait]
impl<V: MaybeVersioned + 'static> ConnectionBuilder<V> for TcpServer {
    async fn build(&self) -> Result<(Connection<V>, ConnectionHandler)> {
        let server_addr = self.addr;
        let listener = TcpListener::bind(self.addr).await?;

        let conn_state = Closer::new();
        let (connection, chan_factory) = Connection::new(self.info.clone(), conn_state.to_shared());

        let handler = ConnectionHandler::spawn(async move {
            on_close_handler(conn_state.to_closable(), server_addr);

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

    fn to_conf(&self) -> AsyncConnConf<V> {
        AsyncConnConf::new(self.clone())
    }
}

fn on_close_handler(state: Closable, addr: SocketAddr) {
    tokio::spawn(async move {
        while !state.is_closed() {
            tokio::time::sleep(SERVER_HANG_UP_TIMEOUT).await;
        }
        _ = TcpStream::connect(addr).await;
    });
}
