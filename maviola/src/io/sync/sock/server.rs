use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::thread;

use mavio::protocol::MaybeVersioned;

use crate::io::sync::connection::{ConnectionBuilder, ConnectionConf, PeerConnection};
use crate::io::{Connection, ConnectionInfo, PeerConnectionInfo};

use crate::prelude::*;

/// Unix socket server configuration.
///
/// Socket server creates a Unix socket on Unix-like systems and starts listening for incoming
/// connections.
#[derive(Clone, Debug)]
pub struct SockServerConf {
    path: PathBuf,
    info: ConnectionInfo,
}

impl SockServerConf {
    /// Instantiates a Unix socket server configuration.
    ///
    /// Accepts as `path` anything that can be converted to [`PathBuf`], validates that path does
    /// not exist.
    pub fn new(path: impl Into<PathBuf>) -> Result<Self> {
        let path: PathBuf = path.into();

        if Path::exists(path.as_path()) {
            return Err(Error::from(std::io::Error::new(
                std::io::ErrorKind::AddrInUse,
                format!("socket path already exists: {path:?}"),
            )));
        }

        let info = ConnectionInfo::SockServer { path: path.clone() };
        Ok(Self { path, info })
    }
}

impl<V: MaybeVersioned + 'static> ConnectionBuilder<V> for SockServerConf {
    fn build(&self) -> Result<Connection<V>> {
        let listener = UnixListener::bind(self.path.as_path())?;
        let path = self.path.clone();

        let (send_tx, send_rx) = mpmc::channel();
        let (recv_tx, recv_rx) = mpmc::channel();

        let conn_info = ConnectionInfo::SockServer { path: path.clone() };
        let connection = Connection::new(conn_info.clone(), send_tx.clone(), recv_rx);

        thread::spawn(move || {
            for stream in listener.incoming() {
                let send_tx = send_tx.clone();
                let send_rx = send_rx.clone();
                let recv_tx = recv_tx.clone();
                let path = path.clone();

                match stream {
                    Ok(writer) => {
                        let reader = match writer.try_clone() {
                            Ok(reader) => reader,
                            Err(err) => {
                                log::error!("[{conn_info:?}] broken incoming stream: {err:?}");
                                return;
                            }
                        };

                        PeerConnection {
                            info: PeerConnectionInfo::SockServer { path },
                            reader,
                            writer,
                            send_tx,
                            send_rx,
                            recv_tx,
                        }
                        .start();
                    }
                    Err(err) => {
                        log::error!("[{conn_info:?}] server failure: {err:?}");
                        return;
                    }
                }
            }
        });

        Ok(connection)
    }
}

impl<V: MaybeVersioned + 'static> ConnectionConf<V> for SockServerConf {
    fn info(&self) -> &ConnectionInfo {
        &self.info
    }
}
