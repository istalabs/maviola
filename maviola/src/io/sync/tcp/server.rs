//! Synchronous TCP server.

use std::net::{SocketAddr, TcpListener, ToSocketAddrs};
use std::sync::atomic::AtomicUsize;
use std::sync::{atomic, mpsc, Arc, Mutex};
use std::thread;

use crate::errors::{Error, Result};
use crate::io::sync::connection::{
    ConnectionBuilder, ConnectionConf, ConnectionEvent, ConnectionInfo,
};
use crate::io::sync::tcp::connection::{TcpConnection, TcpReceiver, TcpSender};
use crate::io::utils::resolve_socket_addr;

/// TCP server configuration.
#[derive(Clone, Debug)]
pub struct TcpServerConf {
    addr: SocketAddr,
}

impl TcpServerConf {
    /// Instantiates a TCP server configuration.
    ///
    /// Accepts as `addr` anything that implements [`ToSocketAddrs`], prefers IPv4 addresses if
    /// available.
    pub fn new(addr: impl ToSocketAddrs) -> Result<Self> {
        Ok(Self {
            addr: resolve_socket_addr(addr)?,
        })
    }
}

impl ConnectionBuilder for TcpServerConf {
    /// Instantiates a TCP server and listens to incoming connections.
    ///
    /// All new connections are sent over [`mpsc::channel`].
    fn build(&self) -> Result<mpsc::Receiver<ConnectionEvent>> {
        let listener = TcpListener::bind(self.addr)?;
        let (tx, rx): (
            mpsc::Sender<ConnectionEvent>,
            mpsc::Receiver<ConnectionEvent>,
        ) = mpsc::channel();
        let server_addr = self.addr;
        let id: AtomicUsize = Default::default();

        thread::spawn(move || {
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        let reader = match stream.try_clone() {
                            Ok(reader) => reader,
                            Err(err) => {
                                let err: Error = err.into();
                                if tx.send(ConnectionEvent::Error(err.clone())).is_err() {
                                    log::error!("unable to pass TCP stream cloning error: {err:?}");
                                    return;
                                }
                                continue;
                            }
                        };

                        let peer_addr = stream.peer_addr().unwrap();
                        let info = ConnectionInfo::TcpServer {
                            server_addr,
                            peer_addr,
                        };
                        let id = id.fetch_add(1, atomic::Ordering::Relaxed);
                        let conn = TcpConnection {
                            id,
                            info: info.clone(),
                            receiver: Arc::new(Mutex::new(Box::new(TcpReceiver {
                                id,
                                receiver: mavio::Receiver::new(reader),
                                event_chan: tx.clone(),
                            }))),
                            sender: Arc::new(Mutex::new(Box::new(TcpSender {
                                id,
                                sender: mavio::Sender::new(stream),
                                event_chan: tx.clone(),
                            }))),
                            event_chan: tx.clone(),
                        };

                        if tx.send(ConnectionEvent::New(Box::new(conn))).is_err() {
                            log::error!("unable to pass connection for: {info:?}");
                            return;
                        }
                    }
                    Err(err) => {
                        let err: Error = err.into();
                        if tx.send(ConnectionEvent::Error(err.clone())).is_err() {
                            log::error!("unable to pass incoming TCP stream error: {err:?}");
                            return;
                        }
                        continue;
                    }
                };
            }
        });

        Ok(rx)
    }
}

impl ConnectionConf for TcpServerConf {}
