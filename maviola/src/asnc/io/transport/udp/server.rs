use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;

use crate::asnc::io::{Connection, ConnectionBuilder, ConnectionHandler};
use crate::asnc::utils::{MpscReader, MpscWriter};
use crate::core::io::{ChannelInfo, ConnectionInfo};
use crate::core::utils::{Closable, Closer};

use crate::prelude::*;

#[async_trait]
impl<V: MaybeVersioned + 'static> ConnectionBuilder<V> for UdpServer {
    async fn build(&self) -> Result<(Connection<V>, ConnectionHandler)> {
        let server_addr = self.addr;
        let udp_socket = Arc::new(UdpSocket::bind(server_addr).await?);

        let conn_state = Closer::new();
        let (connection, chan_factory) = Connection::new(self.info.clone(), conn_state.to_shared());

        let handler = ConnectionHandler::spawn(async move {
            let mut peers = HashMap::new();
            let mut buf = [0u8; 512];

            while !conn_state.is_closed() {
                let (bytes_read, peer_addr) = udp_socket.recv_from(buf.as_mut_slice()).await?;

                #[allow(clippy::map_entry)]
                if !peers.contains_key(&peer_addr) {
                    let udp_socket = udp_socket.clone();

                    let (writer_tx, writer_rx) = mpsc::channel(1024);
                    let (reader_tx, reader_rx) = mpsc::channel(1024);

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
                    channel.spawn().await.discard();

                    Self::handle_async_peer_sends(
                        conn_state.to_closable(),
                        chan_factory.info().clone(),
                        peer_addr,
                        udp_socket,
                        writer_rx,
                    );
                }

                let reader_tx = peers.get(&peer_addr).unwrap();
                reader_tx.send(buf[0..bytes_read].to_vec()).await?;
            }

            Ok(())
        });

        Ok((connection, handler))
    }
}

impl UdpServer {
    fn handle_async_peer_sends(
        conn_state: Closable,
        conn_info: ConnectionInfo,
        peer_addr: SocketAddr,
        udp_socket: Arc<UdpSocket>,
        mut writer_rx: mpsc::Receiver<Vec<u8>>,
    ) {
        tokio::spawn(async move {
            loop {
                if conn_state.is_closed() {
                    return;
                }

                let data = match writer_rx.recv().await {
                    Some(data) => data,
                    None => {
                        log::trace!("[{conn_info:?}] writer channel is closed");
                        return;
                    }
                };
                if let Err(err) = udp_socket.send_to(data.as_slice(), peer_addr).await {
                    log::trace!("[{conn_info:?}] socket is closed: {err:?}");
                    return;
                }
            }
        });
    }
}
