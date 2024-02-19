use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::thread;

use mavio::protocol::MaybeVersioned;

use crate::io::sync::connection::{Connection, ConnectionBuilder, ConnectionConf, PeerConnection};
use crate::io::utils::resolve_socket_addr;
use crate::io::{ConnectionInfo, PeerConnectionInfo};

use crate::prelude::*;

/// TCP client configuration.
///
/// Provides connection configuration for a node that connects to a TCP port as a client.
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
/// # use maviola::TcpServerConf;
/// use maviola::{Event, Node, NodeConf, TcpClientConf};
/// # use maviola::dialects::minimal;
/// # use portpicker::pick_unused_port;
///
/// let addr = "127.0.0.1:5600";
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
/// # {
/// #           let _addr = addr.clone();
///             TcpClientConf::new(addr)    // Configure TCP client connection
/// #           ;TcpServerConf::new(_addr)
/// # }
///                 .unwrap()
///         )
///         .build()
/// ).unwrap();
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct TcpClientConf {
    addr: SocketAddr,
    info: ConnectionInfo,
}

impl TcpClientConf {
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

impl<V: MaybeVersioned + 'static> ConnectionBuilder<V> for TcpClientConf {
    fn build(&self) -> Result<Connection<V>> {
        let server_addr = self.addr;
        let writer = TcpStream::connect(server_addr)?;
        let reader = writer.try_clone()?;

        let (send_tx, send_rx) = mpmc::channel();
        let (recv_tx, recv_rx) = mpmc::channel();

        let connection = Connection::new(self.info.clone(), send_tx.clone(), recv_rx);

        thread::spawn(move || {
            PeerConnection {
                info: PeerConnectionInfo::TcpClient { server_addr },
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

impl<V: MaybeVersioned + 'static> ConnectionConf<V> for TcpClientConf {
    fn info(&self) -> &ConnectionInfo {
        &self.info
    }
}
