use std::net::{SocketAddr, ToSocketAddrs};

use crate::core::io::{ConnectionConf, ConnectionDetails, ConnectionInfo};
use crate::core::utils::net::resolve_socket_addr;

use crate::prelude::*;

/// TCP server configuration.
///
/// Provides connection configuration for a node that binds to a UDP port and communicates with
/// remote UDP connections.
///
/// Each incoming connection will be considered as a separate channel.
///
/// Use [`UdpClient`] to create a TCP client node.
///
/// # Usage
///
/// Create a synchronous UDP server node:
///
/// ```rust,no_run
/// # #[cfg(feature = "sync")] {
/// use maviola::prelude::*;
///
/// let addr = "127.0.0.1:14500";
///
/// // Create a UDP server node
/// let node = Node::sync::<V2>()
///         /* define other node parameters */
/// #       .system_id(1)
/// #       .component_id(1)
///         .connection(
///             UdpServer::new(addr)    // Configure UDP server connection
///                 .unwrap()
///         ).build().unwrap();
/// # }
/// ```
///
/// Create an asynchronous UDP server node:
///
/// ```rust,no_run
/// # #[cfg(not(feature = "async"))] fn main() {}
/// # #[cfg(feature = "async")]
/// # #[tokio::main] async fn main() {
/// use maviola::prelude::*;
///
/// let addr = "127.0.0.1:14500";
///
/// // Create a UDP server node
/// let node = Node::asnc::<V2>()
///         /* define other node parameters */
/// #       .system_id(1)
/// #       .component_id(1)
///         .connection(
///             UdpServer::new(addr)    // Configure UDP server connection
///                 .unwrap()
///         ).build().await.unwrap();
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct UdpServer {
    pub(crate) addr: SocketAddr,
    pub(crate) info: ConnectionInfo,
}

impl UdpServer {
    /// Instantiates a UDP server configuration.
    ///
    /// Accepts as `addr` anything that implements [`ToSocketAddrs`], prefers IPv4 addresses if
    /// available.
    pub fn new(addr: impl ToSocketAddrs) -> Result<Self> {
        let addr = resolve_socket_addr(addr)?;
        let info = ConnectionInfo::new(ConnectionDetails::UdpServer { bind_addr: addr });
        Ok(Self { addr, info })
    }
}

impl ConnectionConf for UdpServer {
    fn info(&self) -> &ConnectionInfo {
        &self.info
    }
}
