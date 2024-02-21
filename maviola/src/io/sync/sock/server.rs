use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::thread;

use mavio::protocol::MaybeVersioned;

use crate::io::sync::connection::{ConnectionBuilder, ConnectionConf};
use crate::io::sync::consts::{SOCK_ACCEPT_INTERVAL, SOCK_READ_TIMEOUT, SOCK_WRITE_TIMEOUT};
use crate::io::sync::utils::handle_listener_stop;
use crate::io::{Connection, ConnectionInfo, PeerConnectionInfo};
use crate::utils::Closer;

use crate::prelude::*;

/// Unix socket server configuration.
///
/// Socket server creates a Unix socket on Unix-like systems and starts listening for incoming
/// connections. Use [`SockClientConf`](super::client::SockClientConf) to create a Unix socket
/// client node.
///
/// Each incoming connection will be considered as a separate channel. You can use
/// [`Callback::respond`](crate::Callback::respond) or
/// [`Callback::respond_others`](crate::Callback::respond_others) to control which channels receive
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
/// use maviola::{Event, Node, SockServerConf};
/// # use maviola::dialects::minimal;
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
/// #         .dialect(minimal::dialect())
///         .connection(
///             SockServerConf::new(path)    // Configure socket server connection
///                 .unwrap()
///         )
/// ).unwrap();
/// # }
/// ```
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
                    PeerConnectionInfo::SockClient { path: path.clone() },
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

impl<V: MaybeVersioned + 'static> ConnectionConf<V> for SockServerConf {
    fn info(&self) -> &ConnectionInfo {
        &self.info
    }
}
