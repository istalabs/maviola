use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::thread;

use mavio::protocol::MaybeVersioned;

use crate::consts::DEFAULT_UDP_HOST;
use crate::io::sync::connection::{ConnectionBuilder, ConnectionConf, PeerConnection};
use crate::io::sync::udp::udp_rw::UdpRW;
use crate::io::utils::{pick_unused_port, resolve_socket_addr};
use crate::io::{Connection, ConnectionInfo, PeerConnectionInfo};

use crate::prelude::*;

/// UDP client configuration.
///
/// Provides connection configuration for a node that communicates with a specified UDP port.
///
/// In UDP-client mode the node will bind to a random port on the system. The host can be set by
/// [`UdpClientConf::with_host`]. By default, the host is equal to [`DEFAULT_UDP_HOST`]. It is also
/// possible to specify exact binding address by [`UdpClientConf::with_bind_addr`].
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
/// use maviola::{Event, Node, NodeConf, UdpClientConf};
/// # use maviola::dialects::minimal;
/// # use portpicker::pick_unused_port;
///
/// let addr = "127.0.0.1:5600";
/// let host = "127.0.0.1";
/// # let addr = format!("127.0.0.1:{}", pick_unused_port().unwrap());
///
/// // Create a TCP client node
/// let node = Node::try_from(
///     NodeConf::builder()
///         /* define other node parameters */
/// #         .version(V2)
/// #         .system_id(1)
/// #         .component_id(1)
/// #         .dialect(minimal::dialect())
///         .connection(
///             UdpClientConf::new(addr)    // Configure UDP client connection
///                 .unwrap()
///                 .with_host(host)        // set bind host (random port will be used for bind addr)
///                 .unwrap()
///         )
///         .build()
/// ).unwrap();
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct UdpClientConf {
    addr: SocketAddr,
    host: String,
    bind_addr: Option<SocketAddr>,
    info: ConnectionInfo,
}

impl UdpClientConf {
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
    /// Discards bind address specified by [`UdpClientConf::with_bind_addr`].
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
    /// [`UdpClientConf::with_host`].
    pub fn with_bind_addr(self, addr: impl ToSocketAddrs) -> Result<Self> {
        Ok(Self {
            addr: self.addr,
            host: self.host,
            bind_addr: Some(resolve_socket_addr(addr)?),
            info: self.info,
        })
    }
}

impl<V: MaybeVersioned + 'static> ConnectionBuilder<V> for UdpClientConf {
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

        let (send_tx, send_rx) = mpmc::channel();
        let (recv_tx, recv_rx) = mpmc::channel();

        let connection = Connection::new(self.info.clone(), send_tx.clone(), recv_rx);

        thread::spawn(move || {
            PeerConnection {
                info: PeerConnectionInfo::UdpClient {
                    server_addr,
                    bind_addr,
                },
                reader,
                writer,
                send_tx,
                send_rx,
                recv_tx,
            }
            .start();
        });

        Ok(connection)
    }
}

impl<V: MaybeVersioned + 'static> ConnectionConf<V> for UdpClientConf {
    fn info(&self) -> &ConnectionInfo {
        &self.info
    }
}
