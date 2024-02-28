use std::net::{SocketAddr, ToSocketAddrs};

use async_trait::async_trait;
use tokio::net::TcpStream;

use crate::asnc::io::{Connection, ConnectionBuilder};
use crate::core::io::{ChannelInfo, ConnectionInfo};
use crate::core::utils::net::resolve_socket_addr;
use crate::core::utils::SharedCloser;

use crate::prelude::*;

/// <sup>[`async`](crate::asnc)</sup>
/// TCP client configuration.
///
/// Provides connection configuration for a node that connects to a TCP port as a client. Use
/// [`TcpServerConf`](super::server::TcpServer) to create a TCP server node.
#[derive(Clone, Debug)]
pub struct TcpClient {
    addr: SocketAddr,
    info: ConnectionInfo,
}

impl TcpClient {
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

#[async_trait]
impl<V: MaybeVersioned + 'static> ConnectionBuilder<V> for TcpClient {
    fn info(&self) -> &ConnectionInfo {
        &self.info
    }

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
