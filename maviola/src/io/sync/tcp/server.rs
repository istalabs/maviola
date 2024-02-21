use std::net::{SocketAddr, TcpListener, ToSocketAddrs};
use std::thread;

use mavio::protocol::MaybeVersioned;

use crate::io::sync::connection::{Connection, ConnectionBuilder, ConnectionConf};
use crate::io::sync::consts::{TCP_READ_TIMEOUT, TCP_WRITE_TIMEOUT};
use crate::io::sync::utils::handle_listener_stop;
use crate::io::utils::resolve_socket_addr;
use crate::io::{ConnectionInfo, PeerConnectionInfo};
use crate::utils::Closer;

use crate::prelude::*;

/// TCP server configuration.
///
/// Provides connection configuration for a node that binds to a TCP port as a server.
///
/// Each incoming connection will be considered as a separate channel. You can use
/// [`Callback::respond`](crate::Callback::respond) or
/// [`Callback::respond_others`](crate::Callback::respond_others) to control which channels receive
/// response messages.
///
/// Use [`TcpClientConf`](super::client::TcpClient) to create a TCP client node.
///
/// # Usage
///
/// Create a TCP server node:
///
/// ```rust
/// # use maviola::protocol::Peer;
/// # #[cfg(feature = "sync")]
/// # {
/// # use maviola::protocol::V2;
/// use maviola::{Event, Node, TcpServer};
/// # use maviola::dialects::minimal;
/// # use portpicker::pick_unused_port;
///
/// let addr = "127.0.0.1:5600";
/// # let addr = format!("127.0.0.1:{}", pick_unused_port().unwrap());
///
/// // Create a TCP server node
/// let node = Node::try_from(
///     Node::builder()
///         /* define other node parameters */
/// #         .version(V2)
/// #         .system_id(1)
/// #         .component_id(1)
/// #         .dialect(minimal::dialect())
///         .connection(
///             TcpServer::new(addr)    // Configure TCP server connection
///                 .unwrap()
///         )
/// ).unwrap();
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct TcpServer {
    addr: SocketAddr,
    info: ConnectionInfo,
}

impl TcpServer {
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

impl<V: MaybeVersioned + 'static> ConnectionBuilder<V> for TcpServer {
    fn build(&self) -> Result<Connection<V>> {
        let server_addr = self.addr;
        let listener = TcpListener::bind(self.addr)?;

        let conn_state = Closer::new();
        let (connection, peer_builder) = Connection::new(self.info.clone(), conn_state.as_shared());

        let handler = thread::spawn(move || -> Result<Closer> {
            for stream in listener.incoming() {
                if conn_state.is_closed() {
                    return Ok(conn_state);
                }

                let stream = stream?;
                let peer_addr = stream.peer_addr()?;
                let writer = stream;
                let reader = writer.try_clone()?;

                writer.set_write_timeout(TCP_WRITE_TIMEOUT)?;
                writer.set_read_timeout(TCP_READ_TIMEOUT)?;

                let peer_connection = peer_builder.build(
                    PeerConnectionInfo::TcpServer {
                        server_addr,
                        peer_addr,
                    },
                    reader,
                    writer,
                );
                peer_connection.spawn().discard();
            }
            Ok(conn_state)
        });

        handle_listener_stop(handler, connection.info().clone());

        Ok(connection)
    }
}

impl<V: MaybeVersioned + 'static> ConnectionConf<V> for TcpServer {
    fn info(&self) -> &ConnectionInfo {
        &self.info
    }
}
