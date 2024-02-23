use std::net::{SocketAddr, ToSocketAddrs};

use async_trait::async_trait;
use tokio::net::TcpListener;

use crate::asnc::conn::{AsyncConnection, AsyncConnectionBuilder};
use crate::asnc::utils::handle_listener_stop;
use crate::core::io::{ChannelInfo, ConnectionInfo};
use crate::core::utils::net::resolve_socket_addr;
use crate::core::utils::Closer;

use crate::prelude::*;

/// TCP server configuration.
///
/// Provides connection configuration for a node that binds to a TCP port as a server.
///
/// Each incoming connection will be considered as a separate channel. You can use
/// [`Callback::respond`](crate::sync::Callback::respond) or
/// [`Callback::respond_others`](crate::sync::Callback::respond_others) to control which channels
/// receive response messages.
///
/// Use [`TcpClientConf`](super::client::AsyncTcpClient) to create a TCP client node.
#[derive(Clone, Debug)]
pub struct AsyncTcpServer {
    addr: SocketAddr,
    info: ConnectionInfo,
}

impl AsyncTcpServer {
    /// Instantiates a TCP server configuration.
    ///
    /// Accepts as `addr` anything that implements [`ToSocketAddrs`], prefers IPv4 addresses if
    /// available.
    pub fn new(addr: impl ToSocketAddrs) -> Result<Self> {
        let addr = resolve_socket_addr(addr)?;
        let info = ConnectionInfo::TcpServer { bind_addr: addr };
        Ok(Self { addr, info })
    }
}

#[async_trait]
impl<V: MaybeVersioned + 'static> AsyncConnectionBuilder<V> for AsyncTcpServer {
    fn info(&self) -> &ConnectionInfo {
        &self.info
    }

    async fn build(&self) -> Result<AsyncConnection<V>> {
        let server_addr = self.addr;
        let listener = TcpListener::bind(self.addr).await?;

        let conn_state = Closer::new();
        let (connection, peer_builder) =
            AsyncConnection::new(self.info.clone(), conn_state.as_shared());

        let handler: tokio::task::JoinHandle<Result<Closer>> = tokio::spawn(async move {
            loop {
                if conn_state.is_closed() {
                    return Ok(conn_state);
                }

                let (stream, peer_addr) = listener.accept().await?;

                let (reader, writer) = stream.into_split();

                let peer_connection = peer_builder.build(
                    ChannelInfo::TcpServer {
                        server_addr,
                        peer_addr,
                    },
                    reader,
                    writer,
                );
                peer_connection.spawn().await.discard();
            }
        });

        handle_listener_stop(handler, connection.info().clone());

        Ok(connection)
    }
}
