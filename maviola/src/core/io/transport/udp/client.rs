use std::net::{SocketAddr, ToSocketAddrs};

use crate::core::consts::DEFAULT_UDP_HOST;
use crate::core::io::{ConnectionConf, ConnectionInfo};
use crate::core::utils::net::resolve_socket_addr;

use crate::prelude::*;

/// UDP client configuration.
///
/// Provides connection configuration for a node that communicates with a specified UDP port. Use
/// [`UdpServer`] to create a UDP server node.
///
/// In UDP-client mode the node will bind to a random port on the system. The host can be set by
/// [`UdpClient::with_host`]. By default, the host is equal to [`DEFAULT_UDP_HOST`]. It is also
/// possible to specify exact binding address by [`UdpClient::with_bind_addr`].
///
/// # Usage
///
/// Create a synchronous UDP client node:
///
/// ```rust,no_run
/// use maviola::prelude::*;
///
/// let addr = "127.0.0.1:14500";
/// let host = "127.0.0.1";
///
/// // Create a UDP client node
/// let node = Node::builder()
///         /* define other node parameters */
/// #       .version::<V2>()
/// #       .system_id(1)
/// #       .component_id(1)
///         .connection(
///             UdpClient::new(addr)    // Configure UDP client connection
///                 .unwrap()
///                 .with_host(host)        // set bind host (random port will be used for bind addr)
///                 .unwrap()
///         ).build().unwrap();
/// ```
///
/// Create an asynchronous UDP client node:
///
/// ```rust,no_run
/// # #[tokio::main] async fn main() {
/// use maviola::prelude::*;
///
/// let addr = "127.0.0.1:14500";
/// let host = "127.0.0.1";
///
/// // Create a UDP client node
/// let node = Node::builder()
///         /* define other node parameters */
/// #       .version::<V2>()
/// #       .system_id(1)
/// #       .component_id(1)
///         .async_connection(
///             UdpClient::new(addr)    // Configure UDP client connection
///                 .unwrap()
///                 .with_host(host)        // set bind host (random port will be used for bind addr)
///                 .unwrap()
///         ).build().await.unwrap();
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct UdpClient {
    pub(crate) addr: SocketAddr,
    pub(crate) host: String,
    pub(crate) bind_addr: Option<SocketAddr>,
    pub(crate) info: ConnectionInfo,
}

impl UdpClient {
    /// Instantiates a UDP client configuration.
    ///
    /// Accepts as `addr` anything that implements [`ToSocketAddrs`], prefers IPv4 addresses if
    /// available.
    pub fn new(addr: impl ToSocketAddrs) -> Result<Self> {
        let addr = resolve_socket_addr(addr)?;
        let info = ConnectionInfo::UdpClient { remote_addr: addr };
        let host = DEFAULT_UDP_HOST.into();
        Ok(Self {
            addr,
            host,
            bind_addr: None,
            info,
        })
    }

    /// Adds host to configuration.
    ///
    /// Discards bind address specified by [`UdpClient::with_bind_addr`].
    pub fn with_host(self, host: impl ToString) -> Result<Self> {
        let host = host.to_string();
        resolve_socket_addr(format!("{host}:80"))?;

        Ok(Self {
            addr: self.addr,
            host: host.to_string(),
            bind_addr: None,
            info: self.info,
        })
    }

    /// Adds a specific binding address to configuration.
    ///
    /// If specified, the binding address will have higher priority over host specified by
    /// [`UdpClient::with_host`].
    pub fn with_bind_addr(self, addr: impl ToSocketAddrs) -> Result<Self> {
        Ok(Self {
            addr: self.addr,
            host: self.host,
            bind_addr: Some(resolve_socket_addr(addr)?),
            info: self.info,
        })
    }
}

impl ConnectionConf for UdpClient {
    fn info(&self) -> &ConnectionInfo {
        &self.info
    }
}
