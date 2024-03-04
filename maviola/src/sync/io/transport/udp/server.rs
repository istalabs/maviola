use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::sync::mpsc;
use std::thread;

use crate::core::io::{ChannelInfo, ConnectionInfo};
use crate::core::utils::{Closable, Closer};
use crate::sync::io::{Connection, ConnectionBuilder};
use crate::sync::utils::{handle_listener_stop, MpscReader, MpscWriter};

use crate::prelude::*;

impl<V: MaybeVersioned + 'static> ConnectionBuilder<V> for UdpServer {
    fn build(&self) -> Result<Connection<V>> {
        let server_addr = self.addr;
        let udp_socket = UdpSocket::bind(server_addr)?;

        let conn_state = Closer::new();
        let (connection, chan_factory) = Connection::new(self.info.clone(), conn_state.to_shared());

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

                    let channel = chan_factory.build(
                        ChannelInfo::UdpServer {
                            server_addr,
                            peer_addr,
                        },
                        reader,
                        writer,
                    );
                    channel.spawn().discard();

                    Self::handle_peer_sends(
                        conn_state.to_closable(),
                        chan_factory.info().clone(),
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
