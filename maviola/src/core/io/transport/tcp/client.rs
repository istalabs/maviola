use std::net::{SocketAddr, ToSocketAddrs};

use crate::core::io::{ConnectionConf, ConnectionInfo};
use crate::core::utils::net::resolve_socket_addr;

use crate::prelude::*;

/// TCP client configuration.
///
/// Provides connection configuration for a node that connects to a TCP port as a client. Use
/// [`TcpServer`] to create a TCP server node.
///
/// # Usage
///
/// Create a synchronous TCP client node:
///
/// ```rust,no_run
/// use maviola::prelude::*;
///
/// let addr = "127.0.0.1:5600";
///
/// // Create a TCP client node
/// let node = Node::builder()
///         /* define other node parameters */
/// #       .version(V2)
/// #       .system_id(1)
/// #       .component_id(1)
///         .connection(
///             TcpClient::new(addr)    // Configure TCP client connection
///                 .unwrap()
///         ).build().unwrap();
/// ```
///
/// Create an asynchronous TCP client node:
///
/// ```rust,no_run
/// # #[tokio::main] async fn main() {
/// use maviola::prelude::*;
///
/// let addr = "127.0.0.1:5600";
///
/// // Create a TCP client node
/// let node = Node::builder()
///         /* define other node parameters */
/// #       .version(V2)
/// #       .system_id(1)
/// #       .component_id(1)
///         .async_connection(
///             TcpClient::new(addr)    // Configure TCP client connection
///                 .unwrap()
///         ).build().await.unwrap();
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct TcpClient {
    pub(crate) addr: SocketAddr,
    pub(crate) info: ConnectionInfo,
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

impl ConnectionConf for TcpClient {
    fn info(&self) -> &ConnectionInfo {
        &self.info
    }
}