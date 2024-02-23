use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::thread;

use crate::protocol::MaybeVersioned;

use crate::io::sync::conn::{Connection, ConnectionBuilder};
use crate::io::sync::consts::{SOCK_ACCEPT_INTERVAL, SOCK_READ_TIMEOUT, SOCK_WRITE_TIMEOUT};
use crate::io::sync::utils::handle_listener_stop;
use crate::io::{ChannelInfo, ConnectionInfo};
use crate::utils::Closer;

use crate::prelude::*;

/// Unix socket server configuration.
///
/// Socket server creates a Unix socket on Unix-like systems and starts listening for incoming
/// connections. Use [`SockClientConf`](super::client::SockClient) to create a Unix socket
/// client node.
///
/// Each incoming connection will be considered as a separate channel. You can use
/// [`Callback::respond`](crate::io::Callback::respond) or
/// [`Callback::respond_others`](crate::io::Callback::respond_others) to control which channels receive
/// response messages.
///
/// # Usage
///
/// Create a Unix-socket server node:
///
/// ```no_run
/// # use std::fs::remove_file;
/// # use std::path::{Path, PathBuf};
/// # #[cfg(feature = "sync")]
/// # #[cfg(unix)]
/// # {
/// # use maviola::protocol::V2;
/// use maviola::{Event, Node, SockServer};
///
/// let path = "/tmp/maviola.sock";
///
/// // Create a Unix-socket server node
/// let node = Node::try_from(
///     Node::builder()
///         /* define other node parameters */
/// #         .version(V2)
/// #         .system_id(1)
/// #         .component_id(1)
///         .connection(
///             SockServer::new(path)    // Configure socket server connection
///                 .unwrap()
///         )
/// ).unwrap();
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct SockServer {
    path: PathBuf,
    info: ConnectionInfo,
}

impl SockServer {
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

impl<V: MaybeVersioned + 'static> ConnectionBuilder<V> for SockServer {
    fn info(&self) -> &ConnectionInfo {
        &self.info
    }

    fn build(&self) -> Result<Connection<V>> {
        let path = self.path.clone();
        let listener = UnixListener::bind(self.path.as_path())?;
        listener.set_nonblocking(true)?;

        let conn_state = Closer::new();
        let (connection, peer_builder) = Connection::new(self.info.clone(), conn_state.as_shared());

        let handler = thread::spawn(move || -> Result<Closer> {
            loop {
                if conn_state.is_closed() {
                    return Ok(conn_state);
                }

                let writer = match listener.accept() {
                    Ok((stream, _)) => stream,
                    Err(err) => match err.kind() {
                        std::io::ErrorKind::WouldBlock => {
                            thread::sleep(SOCK_ACCEPT_INTERVAL);
                            continue;
                        }
                        _ => return Err(err.into()),
                    },
                };
                let reader = writer.try_clone()?;

                writer.set_nonblocking(false)?;
                writer.set_write_timeout(SOCK_WRITE_TIMEOUT)?;
                reader.set_read_timeout(SOCK_READ_TIMEOUT)?;

                let peer_connection = peer_builder.build(
                    ChannelInfo::SockClient { path: path.clone() },
                    reader,
                    writer,
                );
                peer_connection.spawn().discard();
            }
        });

        handle_listener_stop(handler, connection.info().clone());

        Ok(connection)
    }
}
