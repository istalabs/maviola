use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};

use mavio::protocol::MaybeVersioned;

use crate::io::sync::connection::{ConnectionBuilder, ConnectionConf};
use crate::io::{Connection, ConnectionInfo, PeerConnectionInfo};
use crate::utils::SharedCloser;

use crate::prelude::*;

/// Unix socket client configuration.
///
/// Socket client connects to an existing Unix socket on Unix-like systems. Use
/// [`SockServerConf`](super::server::SockServerConf) to create a Unix socket server node.
///
/// # Usage
///
/// Create a Unix-socket server node:
///
/// ```no_run
/// # #[cfg(feature = "sync")]
/// # #[cfg(unix)]
/// # {
/// # use maviola::protocol::V2;
/// use maviola::{Event, Node, SockClientConf};
/// # use maviola::dialects::minimal;
///
/// let path = "/tmp/maviola.sock";
///
/// // Create a Unix-socket client node
/// let node = Node::try_from(
///     Node::builder()
///         /* define other node parameters */
/// #         .version(V2)
/// #         .system_id(1)
/// #         .component_id(1)
/// #         .dialect(minimal::dialect())
///         .connection(
///             SockClientConf::new(path)    // Configure socket server connection
///                 .unwrap()
///         )
/// ).unwrap();
/// # }
/// ```
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
        let path = self.path.clone();
        let writer = UnixStream::connect(path.as_path())?;
        let reader = writer.try_clone()?;

        let conn_state = SharedCloser::new();
        let (connection, peer_builder) = Connection::new(self.info.clone(), conn_state);

        let peer_connection =
            peer_builder.build(PeerConnectionInfo::SockClient { path }, reader, writer);
        peer_connection.spawn().as_closable();

        Ok(connection)
    }
}

impl<V: MaybeVersioned + 'static> ConnectionConf<V> for SockClientConf {
    fn info(&self) -> &ConnectionInfo {
        &self.info
    }
}
