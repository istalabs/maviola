use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::thread;

use mavio::protocol::MaybeVersioned;

use crate::io::sync::connection::{
    Connection, ConnectionBuilder, ConnectionConf, ConnectionInfo, PeerConnection,
    PeerConnectionInfo,
};
use crate::io::utils::resolve_socket_addr;

use crate::prelude::*;

/// TCP client configuration.
#[derive(Clone, Debug)]
pub struct TcpClientConf {
    addr: SocketAddr,
    info: ConnectionInfo,
}

impl TcpClientConf {
    /// Instantiates a TCP client configuration.
    ///
    /// Accepts as `addr` anything that implements [`ToSocketAddrs`], prefers IPv4 addresses if
    /// available.
    pub fn new(addr: impl ToSocketAddrs) -> Result<Self> {
        let addr = resolve_socket_addr(addr)?;
        let info = ConnectionInfo::TcpClient {
            remote_addr: addr.clone(),
        };
        Ok(Self { addr, info })
    }
}

impl<V: MaybeVersioned + 'static> ConnectionBuilder<V> for TcpClientConf {
    fn build(&self) -> Result<Connection<V>> {
        let server_addr = self.addr;
        let writer = TcpStream::connect(server_addr)?;
        let reader = writer.try_clone()?;

        let (send_tx, send_rx) = mpmc::channel();
        let (recv_tx, recv_rx) = mpmc::channel();

        let connection = Connection::new(self.info.clone(), send_tx.clone(), recv_rx);

        thread::spawn(move || {
            PeerConnection {
                info: PeerConnectionInfo::TcpClient { server_addr },
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

impl<V: MaybeVersioned + 'static> ConnectionConf<V> for TcpClientConf {
    fn info(&self) -> &ConnectionInfo {
        &self.info
    }
}
