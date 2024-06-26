use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;

use crate::asnc::io::{Connection, ConnectionBuilder, ConnectionHandler};
use crate::asnc::marker::AsyncConnConf;
use crate::asnc::utils::{MpscReader, MpscWriter};
use crate::core::consts::{DEFAULT_UDP_HOST, SERVER_HANG_UP_TIMEOUT};
use crate::core::io::{ChannelDetails, ConnectionConf, ConnectionInfo};
use crate::core::utils::net::{pick_unused_port, resolve_socket_addr};
use crate::core::utils::{Closable, Closer};

use crate::prelude::*;

#[async_trait]
impl<V: MaybeVersioned> ConnectionBuilder<V> for UdpServer {
    async fn build(&self) -> Result<(Connection<V>, ConnectionHandler)> {
        let server_addr = self.addr;
        let udp_socket = Arc::new(UdpSocket::bind(server_addr).await?);

        let conn_state = Closer::new();
        let (connection, chan_factory) = Connection::new(self.info.clone(), conn_state.to_shared());

        let info = self.info().clone();
        let on_close_bind_addr =
            resolve_socket_addr(format!("{}:{}", DEFAULT_UDP_HOST, pick_unused_port()?))?;

        let handler = ConnectionHandler::spawn(async move {
            on_close_handler(
                conn_state.to_closable(),
                on_close_bind_addr,
                server_addr,
                info.clone(),
            );

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

                    let chan_info = info.make_channel_info(ChannelDetails::UdpServer {
                        server_addr,
                        peer_addr,
                    });
                    let channel = chan_factory.build(chan_info, reader, writer);
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

    fn to_conf(&self) -> AsyncConnConf<V> {
        AsyncConnConf::new(self.clone())
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

fn on_close_handler(
    state: Closable,
    bind_addr: SocketAddr,
    server_addr: SocketAddr,
    info: ConnectionInfo,
) {
    tokio::spawn(async move {
        while !state.is_closed() {
            tokio::time::sleep(SERVER_HANG_UP_TIMEOUT).await;
        }

        log::debug!("[{info:?}] spawn wake-up connection to close server listening loop");
        if let Ok(socket) = UdpSocket::bind(bind_addr).await {
            _ = socket.connect(server_addr).await;
        }
    });
}
