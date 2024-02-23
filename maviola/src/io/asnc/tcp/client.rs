use std::net::{SocketAddr, ToSocketAddrs};

use tokio::net::TcpStream;

use crate::io::asnc::conn::{AsyncConnection, AsyncConnectionBuilder};
use crate::io::utils::resolve_socket_addr;
use crate::io::{ChannelInfo, ConnectionInfo};
use crate::utils::SharedCloser;

use crate::prelude::*;

/// TCP client configuration.
///
/// Provides connection configuration for a node that connects to a TCP port as a client. Use
/// [`TcpServerConf`](super::server::AsyncTcpServer) to create a TCP server node.
#[derive(Clone, Debug)]
pub struct AsyncTcpClient {
    addr: SocketAddr,
    info: ConnectionInfo,
}

impl AsyncTcpClient {
    /// Instantiates a TCP client configuration.
    ///
    /// Accepts as `addr` anything that implements [`ToSocketAddrs`], prefers IPv4 addresses if
    /// available.
    pub fn new(addr: impl ToSocketAddrs) -> Result<Self> {
        let addr = resolve_socket_addr(addr)?;
        let info = ConnectionInfo::TcpClient { remote_addr: addr };
        Ok(Self { addr, info })
    }
}

impl<V: MaybeVersioned + 'static> AsyncConnectionBuilder<V> for AsyncTcpClient {
    fn info(&self) -> &ConnectionInfo {
        &self.info
    }

    async fn build(&self) -> Result<AsyncConnection<V>> {
        let server_addr = self.addr;
        let stream = TcpStream::connect(server_addr).await?;
        let (reader, writer) = stream.into_split();

        let conn_state = SharedCloser::new();
        let (connection, peer_builder) = AsyncConnection::new(self.info.clone(), conn_state);

        let peer_connection =
            peer_builder.build(ChannelInfo::TcpClient { server_addr }, reader, writer);
        peer_connection.spawn().await.discard();

        Ok(connection)
    }
}
