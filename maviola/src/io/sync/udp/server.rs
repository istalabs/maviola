use mavio::Frame;
use std::collections::HashMap;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::sync::mpsc;
use std::thread;

use mavio::protocol::MaybeVersioned;

use crate::io::sync::connection::{ConnectionBuilder, ConnectionConf, PeerConnection};
use crate::io::sync::mpsc_rw::{MpscReader, MpscWriter};
use crate::io::sync::response::ResponseFrame;
use crate::io::utils::resolve_socket_addr;
use crate::io::{Connection, ConnectionInfo, PeerConnectionInfo, Response};

use crate::prelude::*;

/// Synchronous TCP server configuration.
#[derive(Clone, Debug)]
pub struct UdpServerConf {
    addr: SocketAddr,
    info: ConnectionInfo,
}

impl UdpServerConf {
    /// Instantiates a UDP server configuration.
    ///
    /// Accepts as `addr` anything that implements [`ToSocketAddrs`], prefers IPv4 addresses if
    /// available.
    pub fn new(addr: impl ToSocketAddrs) -> Result<Self> {
        let addr = resolve_socket_addr(addr)?;
        let info = ConnectionInfo::UdpServer {
            bind_addr: addr.clone(),
        };
        Ok(Self { addr, info })
    }
}

impl<V: MaybeVersioned + 'static> ConnectionBuilder<V> for UdpServerConf {
    fn build(&self) -> Result<Connection<V>> {
        let server_addr = self.addr;
        let udp_socket = UdpSocket::bind(server_addr)?;

        let (send_tx, send_rx) = mpmc::channel();
        let (recv_tx, recv_rx) = mpmc::channel();

        let conn_info = self.info.clone();
        let connection = Connection::new(self.info.clone(), send_tx.clone(), recv_rx);

        thread::spawn(move || {
            let mut peers = HashMap::new();
            let mut buf = [0u8; 512];

            loop {
                {
                    let received = udp_socket.recv_from(buf.as_mut_slice());
                    let (bytes_read, peer_addr) = match received {
                        Ok((bytes_read, peer_addr)) => (bytes_read, peer_addr),
                        Err(err) => {
                            log::error!(
                                "[{conn_info:?}] unable to receive from UDP socket: {err:?}"
                            );
                            break;
                        }
                    };

                    if !peers.contains_key(&peer_addr) {
                        let udp_socket = match udp_socket.try_clone() {
                            Ok(udp_socket) => udp_socket,
                            Err(err) => {
                                log::error!("[{conn_info:?}] unable to clone UDP socket: {err:?}");
                                break;
                            }
                        };

                        Self::add_peer_connection(
                            conn_info.clone(),
                            server_addr,
                            peer_addr,
                            &mut peers,
                            udp_socket,
                            send_tx.clone(),
                            send_rx.clone(),
                            recv_tx.clone(),
                        );
                    }

                    let reader_tx = peers.get(&peer_addr).unwrap();

                    if let Err(err) = reader_tx.send(buf[0..bytes_read].to_vec()) {
                        log::error!(
                            "[{conn_info:?}] unable to pass incoming UDP datagram: {err:?}"
                        );
                        break;
                    }
                }
            }
        });

        Ok(connection)
    }
}

impl UdpServerConf {
    fn add_peer_connection<V: MaybeVersioned + 'static>(
        conn_info: ConnectionInfo,
        server_addr: SocketAddr,
        peer_addr: SocketAddr,
        peers: &mut HashMap<SocketAddr, mpsc::Sender<Vec<u8>>>,
        udp_socket: UdpSocket,
        send_tx: mpmc::Sender<ResponseFrame<V>>,
        send_rx: mpmc::Receiver<ResponseFrame<V>>,
        recv_tx: mpmc::Sender<(Frame<V>, Response<V>)>,
    ) {
        let (writer_tx, writer_rx) = mpsc::channel();
        let (reader_tx, reader_rx) = mpsc::channel();

        peers.insert(peer_addr, reader_tx);

        let writer = MpscWriter::new(writer_tx);
        let reader = MpscReader::new(reader_rx);

        PeerConnection {
            info: PeerConnectionInfo::UdpServer {
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

        Self::handle_peer_sends(conn_info, peer_addr, udp_socket, writer_rx);
    }

    fn handle_peer_sends(
        conn_info: ConnectionInfo,
        peer_addr: SocketAddr,
        udp_socket: UdpSocket,
        writer_rx: mpsc::Receiver<Vec<u8>>,
    ) {
        thread::spawn(move || loop {
            let data = match writer_rx.recv() {
                Ok(data) => data,
                Err(err) => {
                    log::error!("[{conn_info:?}] UDP writer channel is closed: {err:?}");
                    return;
                }
            };
            if let Err(err) = udp_socket.send_to(data.as_slice(), peer_addr) {
                log::error!("[{conn_info:?}] UDP socket is closed: {err:?}");
                return;
            }
        });
    }
}

impl<V: MaybeVersioned + 'static> ConnectionConf<V> for UdpServerConf {
    fn info(&self) -> &ConnectionInfo {
        &self.info
    }
}
