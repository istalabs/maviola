use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::thread;

use mavio::protocol::MaybeVersioned;

use crate::io::sync::connection::{ConnectionBuilder, ConnectionConf, PeerConnection};
use crate::io::sync::udp::udp_rw::UdpRW;
use crate::io::utils::{pick_unused_port, resolve_socket_addr};
use crate::io::{Connection, ConnectionInfo, PeerConnectionInfo};

use crate::prelude::*;

/// Synchronous UDP client configuration.
#[derive(Clone, Debug)]
pub struct UdpClientConf {
    addr: SocketAddr,
    info: ConnectionInfo,
}

impl UdpClientConf {
    /// Instantiates a UDP client configuration.
    ///
    /// Accepts as `addr` anything that implements [`ToSocketAddrs`], prefers IPv4 addresses if
    /// available.
    pub fn new(addr: impl ToSocketAddrs) -> Result<Self> {
        let addr = resolve_socket_addr(addr)?;
        let info = ConnectionInfo::UdpClient {
            remote_addr: addr.clone(),
        };
        Ok(Self { addr, info })
    }
}

impl<V: MaybeVersioned + 'static> ConnectionBuilder<V> for UdpClientConf {
    fn build(&self) -> Result<Connection<V>> {
        let bind_addr = resolve_socket_addr(format!("127.0.0.1:{}", pick_unused_port()?))?;
        let server_addr = self.addr;

        let udp_socket = UdpSocket::bind(bind_addr)?;
        udp_socket.connect(server_addr)?;

        let writer = UdpRW::new(udp_socket);
        let reader = writer.try_clone()?;

        let (send_tx, send_rx) = mpmc::channel();
        let (recv_tx, recv_rx) = mpmc::channel();

        let connection = Connection::new(self.info.clone(), send_tx.clone(), recv_rx);

        thread::spawn(move || {
            PeerConnection {
                info: PeerConnectionInfo::UdpClient {
                    server_addr,
                    bind_addr,
                },
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

impl<V: MaybeVersioned + 'static> ConnectionConf<V> for UdpClientConf {
    fn info(&self) -> &ConnectionInfo {
        &self.info
    }
}
