use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};

use crate::consts::DEFAULT_UDP_HOST;
use crate::io::sync::conn::{Connection, ConnectionBuilder};
use crate::io::sync::udp::udp_rw::UdpRW;
use crate::io::utils::{pick_unused_port, resolve_socket_addr};
use crate::io::{ChannelInfo, ConnectionInfo};
use crate::utils::SharedCloser;

use crate::prelude::*;

/// UDP client configuration.
///
/// Provides connection configuration for a node that communicates with a specified UDP port. Use
/// [`UdpServerConf`](super::server::UdpServer) to create a UDP server node.
///
/// In UDP-client mode the node will bind to a random port on the system. The host can be set by
/// [`UdpClient::with_host`]. By default, the host is equal to [`DEFAULT_UDP_HOST`]. It is also
/// possible to specify exact binding address by [`UdpClient::with_bind_addr`].
///
/// # Usage
///
/// Create a TCP client node:
///
/// ```rust
/// # use maviola::protocol::Peer;
/// # #[cfg(feature = "sync")]
/// # {
/// # use maviola::protocol::V2;
/// use maviola::{Event, Node, UdpClient};
/// # use portpicker::pick_unused_port;
///
/// let addr = "127.0.0.1:5600";
/// let host = "127.0.0.1";
/// # let addr = format!("127.0.0.1:{}", pick_unused_port().unwrap());
///
/// // Create a UDP client node
/// let node = Node::try_from(
///     Node::builder()
///         /* define other node parameters */
/// #         .version(V2)
/// #         .system_id(1)
/// #         .component_id(1)
///         .connection(
///             UdpClient::new(addr)    // Configure UDP client connection
///                 .unwrap()
///                 .with_host(host)        // set bind host (random port will be used for bind addr)
///                 .unwrap()
///         )
/// ).unwrap();
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct UdpClient {
    addr: SocketAddr,
    host: String,
    bind_addr: Option<SocketAddr>,
    info: ConnectionInfo,
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

impl<V: MaybeVersioned + 'static> ConnectionBuilder<V> for UdpClient {
    fn info(&self) -> &ConnectionInfo {
        &self.info
    }

    fn build(&self) -> Result<Connection<V>> {
        let bind_addr = match self.bind_addr {
            None => resolve_socket_addr(format!("{}:{}", self.host, pick_unused_port()?))?,
            Some(bind_addr) => bind_addr,
        };
        let server_addr = self.addr;

        let udp_socket = UdpSocket::bind(bind_addr)?;
        udp_socket.connect(server_addr)?;

        let writer = UdpRW::new(udp_socket);
        let reader = writer.try_clone()?;

        let conn_state = SharedCloser::new();
        let (connection, peer_builder) = Connection::new(self.info.clone(), conn_state);

        let peer_connection = peer_builder.build(
            ChannelInfo::UdpClient {
                server_addr,
                bind_addr,
            },
            reader,
            writer,
        );
        peer_connection.spawn().discard();

        Ok(connection)
    }
}
