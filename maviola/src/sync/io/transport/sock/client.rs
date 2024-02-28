use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};

use crate::core::io::{ChannelInfo, ConnectionInfo};
use crate::core::utils::SharedCloser;
use crate::sync::io::{Connection, ConnectionBuilder};

use crate::prelude::*;

/// <sup>[`sync`](crate::sync)</sup>
/// <sup>`unix`</sup>
/// Unix socket client configuration.
///
/// Socket client connects to an existing Unix socket on Unix-like systems. Use
/// [`SockServerConf`](super::server::SockServer) to create a Unix socket server node.
///
/// # Usage
///
/// Create a Unix-socket server node:
///
/// ```no_run
/// use maviola::prelude::*;
/// use maviola::sync::io::SockClient;
///
/// let path = "/tmp/maviola.sock";
///
/// // Create a Unix-socket client node
/// let node = Node::builder()
///         /* define other node parameters */
/// #       .version(V2)
/// #       .system_id(1)
/// #       .component_id(1)
///         .connection(
///             SockClient::new(path)    // Configure socket server connection
///                 .unwrap()
///         ).build().unwrap();
/// ```
#[derive(Clone, Debug)]
pub struct SockClient {
    path: PathBuf,
    info: ConnectionInfo,
}

impl SockClient {
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

impl<V: MaybeVersioned + 'static> ConnectionBuilder<V> for SockClient {
    fn info(&self) -> &ConnectionInfo {
        &self.info
    }

    fn build(&self) -> Result<Connection<V>> {
        let path = self.path.clone();
        let writer = UnixStream::connect(path.as_path())?;
        let reader = writer.try_clone()?;

        let conn_state = SharedCloser::new();
        let (connection, peer_builder) = Connection::new(self.info.clone(), conn_state);

        let peer_connection = peer_builder.build(ChannelInfo::SockClient { path }, reader, writer);
        peer_connection.spawn().discard();

        Ok(connection)
    }
}
