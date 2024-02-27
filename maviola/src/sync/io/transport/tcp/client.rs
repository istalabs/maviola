use std::net::{SocketAddr, TcpStream, ToSocketAddrs};

use crate::core::io::{ChannelInfo, ConnectionInfo};
use crate::core::utils::net::resolve_socket_addr;
use crate::core::utils::SharedCloser;
use crate::sync::io::{Connection, ConnectionBuilder};

use crate::prelude::*;

/// <sup>[`sync`](crate::sync)</sup>
/// TCP client configuration.
///
/// Provides connection configuration for a node that connects to a TCP port as a client. Use
/// [`TcpServerConf`](super::server::TcpServer) to create a TCP server node.
///
/// # Usage
///
/// Create a TCP client node:
///
/// ```no_run
/// use maviola::prelude::*;
/// use maviola::sync::io::TcpClient;
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

impl<V: MaybeVersioned + 'static> ConnectionBuilder<V> for TcpClient {
    fn info(&self) -> &ConnectionInfo {
        &self.info
    }

    fn build(&self) -> Result<Connection<V>> {
        let server_addr = self.addr;
        let writer = TcpStream::connect(server_addr)?;
        let reader = writer.try_clone()?;

        let conn_state = SharedCloser::new();
        let (connection, peer_builder) = Connection::new(self.info.clone(), conn_state);

        let peer_connection =
            peer_builder.build(ChannelInfo::TcpClient { server_addr }, reader, writer);
        peer_connection.spawn().discard();

        Ok(connection)
    }
}