//! Synchronous TCP server.

use std::net::{SocketAddr, TcpListener, ToSocketAddrs};
use std::sync::atomic::AtomicUsize;
use std::sync::{atomic, mpsc, Arc, Mutex};
use std::thread;

use crate::prelude::*;

use crate::io::sync::connection::{
    ConnectionBuilder, ConnectionConf, ConnectionConfInfo, ConnectionEvent, ConnectionInfo,
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
        let conn_conf_info = ConnectionConfInfo::TcpServer {
            bind_addr: server_addr,
        };

        thread::spawn(move || {
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        let reader = match stream.try_clone() {
                            Ok(reader) => reader,
                            Err(err) => {
                                let err: Error = err.into();
                                let conn_conf_info = ConnectionConfInfo::TcpServer {
                                    bind_addr: server_addr,
                                };
                                if tx.send(ConnectionEvent::Error(err.clone())).is_err() {
                                    log::error!("{conn_conf_info:?} unable to pass TCP stream cloning error: {err:?}");
                                    return;
                                }
                                continue;
                            }
                        };

                        let peer_addr = stream.peer_addr().unwrap();
                        let conn_info = ConnectionInfo::TcpServer {
                            server_addr,
                            peer_addr,
                        };
                        let id = id.fetch_add(1, atomic::Ordering::Relaxed);
                        let receiver = TcpReceiver::new(
                            0,
                            conn_info.clone(),
                            tx.clone(),
                            mavio::Receiver::new(reader),
                        );
                        let sender = TcpSender::new(
                            0,
                            conn_info.clone(),
                            tx.clone(),
                            mavio::Sender::new(stream),
                        );
                        let conn = TcpConnection {
                            id,
                            info: conn_info.clone(),
                            receiver: Arc::new(Mutex::new(Box::new(receiver))),
                            sender: Arc::new(Mutex::new(Box::new(sender))),
                            events_chan: tx.clone(),
                        };

                        if let Err(err) = tx.send(ConnectionEvent::New(Box::new(conn))) {
                            log::error!("{conn_info:?} unable to register connection: {err:?}");
                            return;
                        }
                    }
                    Err(err) => {
                        let err: Error = err.into();
                        if tx.send(ConnectionEvent::Error(err.clone())).is_err() {
                            log::error!("{conn_conf_info:?}: unable to pass incoming TCP stream error: {err:?}");
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

impl ConnectionConf for TcpServerConf {
    fn info(&self) -> ConnectionConfInfo {
        ConnectionConfInfo::TcpServer {
            bind_addr: self.addr,
        }
    }
}
