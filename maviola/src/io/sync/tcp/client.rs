//! Synchronous TCP client.

use mavio::protocol::MaybeVersioned;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

use crate::io::sync::connection::{
    ConnectionBuilder, ConnectionConf, ConnectionConfInfo, ConnectionEvent, ConnectionInfo,
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

impl<V: MaybeVersioned + 'static> ConnectionBuilder<V> for TcpClientConf {
    /// Instantiates a TCP client.
    fn build(&self) -> crate::errors::Result<mpsc::Receiver<ConnectionEvent<V>>> {
        let server_addr = self.addr;
        let stream = TcpStream::connect(server_addr)?;
        let reader = stream.try_clone()?;

        let (tx, rx): (
            mpsc::Sender<ConnectionEvent<V>>,
            mpsc::Receiver<ConnectionEvent<V>>,
        ) = mpsc::channel();

        thread::spawn(move || {
            let conn_conf_info = ConnectionInfo::TcpClient { server_addr };
            let receiver = TcpReceiver::new(
                0,
                conn_conf_info.clone(),
                tx.clone(),
                mavio::Receiver::new(reader),
            );
            let sender = TcpSender::new(
                0,
                conn_conf_info.clone(),
                tx.clone(),
                mavio::Sender::new(stream),
            );
            let conn = TcpConnection {
                id: 0,
                info: conn_conf_info.clone(),
                receiver: Arc::new(Mutex::new(Box::new(receiver))),
                sender: Arc::new(Mutex::new(Box::new(sender))),
                events_chan: tx.clone(),
            };

            if let Err(err) = tx.send(ConnectionEvent::New(Box::new(conn))) {
                log::error!("{conn_conf_info:?} unable to register connection: {err:?}");
            }
        });

        Ok(rx)
    }
}

impl<V: MaybeVersioned + 'static> ConnectionConf<V> for TcpClientConf {
    fn info(&self) -> ConnectionConfInfo {
        ConnectionConfInfo::TcpClient {
            remote_addr: self.addr,
        }
    }
}
