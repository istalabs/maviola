use std::collections::HashMap;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::sync::mpsc;
use std::thread;

use crate::core::io::{ChannelInfo, ConnectionInfo};
use crate::core::utils::net::resolve_socket_addr;
use crate::core::utils::{Closable, Closer};
use crate::sync::io::{Connection, ConnectionBuilder};
use crate::sync::utils::{handle_listener_stop, MpscReader, MpscWriter};

use crate::prelude::*;

/// <sup>[`sync`](crate::sync)</sup>
/// TCP server configuration.
///
/// Provides connection configuration for a node that binds to a UDP port and communicates with
/// remote UDP connections.
///
/// Each incoming connection will be considered as a separate channel. You can use
/// [`Callback::respond`](crate::sync::io::Callback::respond) or
/// [`Callback::respond_others`](crate::sync::io::Callback::respond_others) to control which channels
/// receive response messages.
///
/// Use [`UdpClientConf`](super::client::UdpClient) to create a TCP client node.
///
/// # Usage
///
/// Create a UDP server node:
///
/// ```no_run
/// use maviola::prelude::*;
/// use maviola::sync::io::UdpServer;
///
/// let addr = "127.0.0.1:5600";
///
/// // Create a UDP server node
/// let node = Node::builder()
///         /* define other node parameters */
/// #       .version(V2)
/// #       .system_id(1)
/// #       .component_id(1)
///         .connection(
///             UdpServer::new(addr)    // Configure UDP server connection
///                 .unwrap()
///         ).build().unwrap();
/// ```
#[derive(Clone, Debug)]
pub struct UdpServer {
    addr: SocketAddr,
    info: ConnectionInfo,
}

impl UdpServer {
    /// Instantiates a UDP server configuration.
    ///
    /// Accepts as `addr` anything that implements [`ToSocketAddrs`], prefers IPv4 addresses if
    /// available.
    pub fn new(addr: impl ToSocketAddrs) -> Result<Self> {
        let addr = resolve_socket_addr(addr)?;
        let info = ConnectionInfo::UdpServer { bind_addr: addr };
        Ok(Self { addr, info })
    }
}

impl<V: MaybeVersioned + 'static> ConnectionBuilder<V> for UdpServer {
    fn info(&self) -> &ConnectionInfo {
        &self.info
    }

    fn build(&self) -> Result<Connection<V>> {
        let server_addr = self.addr;
        let udp_socket = UdpSocket::bind(server_addr)?;

        let conn_state = Closer::new();
        let (connection, peer_builder) = Connection::new(self.info.clone(), conn_state.to_shared());

        let handler = thread::spawn(move || -> Result<Closer> {
            let mut peers = HashMap::new();
            let mut buf = [0u8; 512];

            loop {
                if conn_state.is_closed() {
                    return Ok(conn_state);
                }

                let (bytes_read, peer_addr) = udp_socket.recv_from(buf.as_mut_slice())?;

                #[allow(clippy::map_entry)]
                if !peers.contains_key(&peer_addr) {
                    let udp_socket = udp_socket.try_clone()?;

                    let (writer_tx, writer_rx) = mpsc::channel();
                    let (reader_tx, reader_rx) = mpsc::channel();

                    peers.insert(peer_addr, reader_tx);

                    let writer = MpscWriter::new(writer_tx);
                    let reader = MpscReader::new(reader_rx);

                    let peer_connection = peer_builder.build(
                        ChannelInfo::UdpServer {
                            server_addr,
                            peer_addr,
                        },
                        reader,
                        writer,
                    );
                    peer_connection.spawn().discard();

                    Self::handle_peer_sends(
                        conn_state.to_closable(),
                        peer_builder.info().clone(),
                        peer_addr,
                        udp_socket,
                        writer_rx,
                    );
                }

                let reader_tx = peers.get(&peer_addr).unwrap();
                reader_tx.send(buf[0..bytes_read].to_vec())?;
            }
        });

        handle_listener_stop(handler, connection.info().clone());

        Ok(connection)
    }
}

impl UdpServer {
    fn handle_peer_sends(
        conn_state: Closable,
        conn_info: ConnectionInfo,
        peer_addr: SocketAddr,
        udp_socket: UdpSocket,
        writer_rx: mpsc::Receiver<Vec<u8>>,
    ) {
        thread::spawn(move || loop {
            if conn_state.is_closed() {
                return;
            }

            let data = match writer_rx.recv() {
                Ok(data) => data,
                Err(err) => {
                    log::trace!("[{conn_info:?}] writer channel is closed: {err:?}");
                    return;
                }
            };
            if let Err(err) = udp_socket.send_to(data.as_slice(), peer_addr) {
                log::trace!("[{conn_info:?}] socket is closed: {err:?}");
                return;
            }
        });
    }
}
