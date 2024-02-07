//! Synchronous TCP client.

use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

use crate::io::sync::connection::{
    ConnectionBuilder, ConnectionConf, ConnectionEvent, ConnectionInfo,
};
use crate::io::sync::tcp::connection::{TcpConnection, TcpReceiver, TcpSender};
use crate::io::utils::resolve_socket_addr;

/// TCP client configuration.
#[derive(Clone, Debug)]
pub struct TcpClientConf {
    addr: SocketAddr,
}

impl TcpClientConf {
    /// Instantiates a TCP client configuration.
    ///
    /// Accepts as `addr` anything that implements [`ToSocketAddrs`], prefers IPv4 addresses if
    /// available.
    pub fn new(addr: impl ToSocketAddrs) -> crate::errors::Result<Self> {
        Ok(Self {
            addr: resolve_socket_addr(addr)?,
        })
    }
}

impl ConnectionBuilder for TcpClientConf {
    /// Instantiates a TCP client.
    fn build(&self) -> crate::errors::Result<mpsc::Receiver<ConnectionEvent>> {
        let server_addr = self.addr;
        let stream = TcpStream::connect(server_addr)?;
        let reader = stream.try_clone()?;

        let (tx, rx): (
            mpsc::Sender<ConnectionEvent>,
            mpsc::Receiver<ConnectionEvent>,
        ) = mpsc::channel();

        thread::spawn(move || {
            let info = ConnectionInfo::TcpClient { server_addr };
            let conn = TcpConnection {
                id: 0,
                info: info.clone(),
                receiver: Arc::new(Mutex::new(Box::new(TcpReceiver {
                    id: 0,
                    receiver: mavio::Receiver::new(reader),
                    event_chan: tx.clone(),
                }))),
                sender: Arc::new(Mutex::new(Box::new(TcpSender {
                    id: 0,
                    sender: mavio::Sender::new(stream),
                    event_chan: tx.clone(),
                }))),
                event_chan: tx.clone(),
            };

            if tx.send(ConnectionEvent::New(Box::new(conn))).is_err() {
                log::error!("unable to send connection for {info:?}");
            }
        });

        Ok(rx)
    }
}

impl ConnectionConf for TcpClientConf {}
