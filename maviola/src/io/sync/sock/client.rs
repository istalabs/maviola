use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::thread;

use mavio::protocol::MaybeVersioned;

use crate::io::sync::connection::{ConnectionBuilder, ConnectionConf, PeerConnection};
use crate::io::{Connection, ConnectionInfo, PeerConnectionInfo};

use crate::prelude::*;

/// Unix socket client configuration.
#[derive(Clone, Debug)]
pub struct SockClientConf {
    path: PathBuf,
    info: ConnectionInfo,
}

impl SockClientConf {
    /// Instantiates a Unix socket client configuration.
    ///
    /// Accepts as `path` anything that can be converted to [`PathBuf`], validates that path exists
    /// and is indeed a file.
    pub fn new(path: impl Into<PathBuf>) -> Result<Self> {
        let path: PathBuf = path.into();

        if !Path::exists(path.as_path()) {
            return Err(Error::from(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("socket does not exists: {path:?}"),
            )));
        }

        let info = ConnectionInfo::SockClient { path: path.clone() };
        Ok(Self { path, info })
    }
}

impl<V: MaybeVersioned + 'static> ConnectionBuilder<V> for SockClientConf {
    fn build(&self) -> Result<Connection<V>> {
        let writer = UnixStream::connect(self.path.as_path())?;
        let reader = writer.try_clone()?;

        let (send_tx, send_rx) = mpmc::channel();
        let (recv_tx, recv_rx) = mpmc::channel();

        let connection = Connection::new(self.info.clone(), send_tx.clone(), recv_rx);
        let path = self.path.clone();

        thread::spawn(move || {
            PeerConnection {
                info: PeerConnectionInfo::SockClient { path },
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

impl<V: MaybeVersioned + 'static> ConnectionConf<V> for SockClientConf {
    fn info(&self) -> &ConnectionInfo {
        &self.info
    }
}
