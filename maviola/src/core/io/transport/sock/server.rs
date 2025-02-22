use std::path::{Path, PathBuf};

use crate::core::io::{ConnectionConf, ConnectionDetails, ConnectionInfo};

use crate::prelude::*;

/// <sup>`unix`</sup>
/// Unix socket server configuration.
///
/// Socket server creates a Unix socket on Unix-like systems and starts listening for incoming
/// connections. Use [`SockClient`] to create a Unix socket client node.
///
/// Each incoming connection will be considered as a separate channel.
///
/// # Usage
///
/// Create a synchronous Unix-socket server node:
///
/// ```rust,no_run
/// # #[cfg(feature = "sync")] {
/// use maviola::prelude::*;
///
/// let path = "/tmp/maviola.sock";
///
/// // Create a Unix-socket server node
/// let node = Node::sync::<V2>()
///         /* define other node parameters */
/// #       .system_id(1)
/// #       .component_id(1)
///         .connection(
///             SockServer::new(path)    // Configure socket server connection
///                 .unwrap()
///         ).build().unwrap();
/// # }
/// ```
///
/// Create an asynchronous Unix-socket server node:
///
/// ```rust,no_run
/// # #[cfg(not(feature = "async"))] fn main() {}
/// # #[cfg(feature = "async")]
/// # #[tokio::main] async fn main() {
/// use maviola::prelude::*;
///
/// let path = "/tmp/maviola.sock";
///
/// // Create a Unix-socket server node
/// let node = Node::asnc::<V2>()
///         /* define other node parameters */
/// #       .system_id(1)
/// #       .component_id(1)
///         .connection(
///             SockServer::new(path)    // Configure socket server connection
///                 .unwrap()
///         ).build().await.unwrap();
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct SockServer {
    pub(crate) path: PathBuf,
    pub(crate) info: ConnectionInfo,
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

        let info = ConnectionInfo::new(ConnectionDetails::SockServer { path: path.clone() });
        Ok(Self { path, info })
    }
}

impl ConnectionConf for SockServer {
    fn info(&self) -> &ConnectionInfo {
        &self.info
    }
}
