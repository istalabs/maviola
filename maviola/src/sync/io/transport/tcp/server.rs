use std::net::{SocketAddr, TcpListener, ToSocketAddrs};
use std::thread;

use crate::core::io::{ChannelInfo, ConnectionInfo};
use crate::core::utils::net::resolve_socket_addr;
use crate::core::utils::Closer;
use crate::sync::consts::{TCP_READ_TIMEOUT, TCP_WRITE_TIMEOUT};
use crate::sync::io::{Connection, ConnectionBuilder};
use crate::sync::utils::handle_listener_stop;

use crate::prelude::*;

/// <sup>[`sync`](crate::sync)</sup>
/// TCP server configuration.
///
/// Provides connection configuration for a node that binds to a TCP port as a server.
///
/// Each incoming connection will be considered as a separate channel. You can use
/// [`Callback::respond`](crate::sync::io::Callback::respond) or
/// [`Callback::respond_others`](crate::sync::io::Callback::respond_others) to control which channels
/// receive response messages.
///
/// Use [`TcpClientConf`](super::client::TcpClient) to create a TCP client node.
///
/// # Usage
///
/// Create a TCP server node:
///
/// ```rust
/// use maviola::prelude::*;
/// use maviola::sync::io::TcpServer;
///
/// let addr = "127.0.0.1:5600";
///
/// // Create a TCP server node
/// let node = Node::builder()
///         /* define other node parameters */
/// #       .version(V2)
/// #       .system_id(1)
/// #       .component_id(1)
///         .connection(
///             TcpServer::new(addr)    // Configure TCP server connection
///                 .unwrap()
///         ).build().unwrap();
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
    fn info(&self) -> &ConnectionInfo {
        &self.info
    }

    fn build(&self) -> Result<Connection<V>> {
        let server_addr = self.addr;
        let listener = TcpListener::bind(self.addr)?;

        let conn_state = Closer::new();
        let (connection, peer_builder) = Connection::new(self.info.clone(), conn_state.to_shared());

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
                    ChannelInfo::TcpServer {
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
