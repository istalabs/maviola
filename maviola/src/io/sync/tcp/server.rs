use std::net::{SocketAddr, TcpListener, ToSocketAddrs};
use std::thread;

use mavio::protocol::MaybeVersioned;

use crate::io::sync::connection::{Connection, ConnectionBuilder, ConnectionConf, PeerConnection};
use crate::io::utils::resolve_socket_addr;
use crate::io::{ConnectionInfo, PeerConnectionInfo};

use crate::prelude::*;

/// TCP server configuration.
///
/// Provides connection configuration for a node that binds to a TCP port as a server.
///
/// Each incoming connection will be considered as a separate channel. You can use
/// [`Response::respond`](crate::Response::respond) or
/// [`Response::respond_others`](crate::Response::respond_others) to control which channels receive
/// response messages.
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
/// use maviola::{Event, Node, NodeConf, TcpServerConf};
/// # use maviola::dialects::minimal;
/// # use portpicker::pick_unused_port;
///
/// let addr = "127.0.0.1:5600";
/// # let addr = format!("127.0.0.1:{}", pick_unused_port().unwrap());
///
/// // Create a TCP server node
/// let node = Node::try_from(
///     NodeConf::builder()
///         /* define other node parameters */
/// #         .version(V2)
/// #         .system_id(1)
/// #         .component_id(1)
/// #         .dialect(minimal::dialect())
///         .connection(
///             TcpServerConf::new(addr)    // Configure TCP server connection
///                 .unwrap()
///         )
///         .build()
/// ).unwrap();
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct TcpServerConf {
    addr: SocketAddr,
    info: ConnectionInfo,
}

impl TcpServerConf {
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

impl<V: MaybeVersioned + 'static> ConnectionBuilder<V> for TcpServerConf {
    fn build(&self) -> Result<Connection<V>> {
        let listener = TcpListener::bind(self.addr)?;
        let server_addr = self.addr;

        let (send_tx, send_rx) = mpmc::channel();
        let (recv_tx, recv_rx) = mpmc::channel();

        let conn_info = ConnectionInfo::TcpServer {
            bind_addr: server_addr,
        };
        let connection = Connection::new(conn_info.clone(), send_tx.clone(), recv_rx);

        thread::spawn(move || {
            for stream in listener.incoming() {
                let send_tx = send_tx.clone();
                let send_rx = send_rx.clone();
                let recv_tx = recv_tx.clone();

                match stream {
                    Ok(stream) => {
                        let peer_addr = stream.peer_addr().unwrap();
                        let writer = stream;
                        let reader = match writer.try_clone() {
                            Ok(reader) => reader,
                            Err(err) => {
                                log::error!("[{conn_info:?}] broken incoming stream: {err:?}");
                                return;
                            }
                        };

                        PeerConnection {
                            info: PeerConnectionInfo::TcpServer {
                                server_addr,
                                peer_addr,
                            },
                            reader,
                            writer,
                            send_tx,
                            send_rx,
                            recv_tx,
                        }
                        .start();
                    }
                    Err(err) => {
                        log::error!("[{conn_info:?}] server failure: {err:?}");
                        return;
                    }
                };
            }
        });

        Ok(connection)
    }
}

impl<V: MaybeVersioned + 'static> ConnectionConf<V> for TcpServerConf {
    fn info(&self) -> &ConnectionInfo {
        &self.info
    }
}
