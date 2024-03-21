use std::net::{SocketAddr, ToSocketAddrs};

use crate::core::io::{ConnectionConf, ConnectionDetails, ConnectionInfo};
use crate::core::utils::net::resolve_socket_addr;

use crate::prelude::*;

/// TCP server configuration.
///
/// Provides connection configuration for a node that binds to a TCP port as a server.
///
/// Each incoming connection will be considered as a separate channel.
///
/// Use [`TcpClient`] to create a TCP client node.
///
/// # Usage
///
/// Create a synchronous TCP server node:
///
/// ```rust,no_run
/// use maviola::prelude::*;
///
/// let addr = "127.0.0.1:5600";
///
/// // Create a TCP server node
/// let node = Node::sync::<V2>()
///         /* define other node parameters */
/// #       .system_id(1)
/// #       .component_id(1)
///         .connection(
///             TcpServer::new(addr)    // Configure TCP server connection
///                 .unwrap()
///         ).build().unwrap();
/// ```
///
/// Create an asynchronous TCP server node:
///
/// ```rust,no_run
/// # #[tokio::main] async fn main() {
/// use maviola::prelude::*;
///
/// let addr = "127.0.0.1:5600";
///
/// // Create a TCP server node
/// let node = Node::asnc::<V2>()
///         /* define other node parameters */
/// #       .system_id(1)
/// #       .component_id(1)
///         .connection(
///             TcpServer::new(addr)    // Configure TCP server connection
///                 .unwrap()
///         ).build().await.unwrap();
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct TcpServer {
    pub(crate) addr: SocketAddr,
    pub(crate) info: ConnectionInfo,
}

impl TcpServer {
    /// Instantiates a TCP server configuration.
    ///
    /// Accepts as `addr` anything that implements [`ToSocketAddrs`], prefers IPv4 addresses if
    /// available.
    pub fn new(addr: impl ToSocketAddrs) -> Result<Self> {
        let addr = resolve_socket_addr(addr)?;
        let info = ConnectionInfo::new(ConnectionDetails::TcpServer { bind_addr: addr });
        Ok(Self { addr, info })
    }
}

impl ConnectionConf for TcpServer {
    fn info(&self) -> &ConnectionInfo {
        &self.info
    }
}
