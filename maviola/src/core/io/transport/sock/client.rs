use std::path::{Path, PathBuf};

use crate::core::io::{ConnectionConf, ConnectionDetails, ConnectionInfo};

use crate::prelude::*;

/// <sup>`unix`</sup>
/// Unix socket client configuration.
///
/// Socket client connects to an existing Unix socket on Unix-like systems. Use [`SockServer`]
/// to create a Unix socket server node.
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
/// // Create a Unix-socket client node
/// let node = Node::sync::<V2>()
///         /* define other node parameters */
/// #       .system_id(1)
/// #       .component_id(1)
///         .connection(
///             SockClient::new(path)    // Configure socket server connection
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
/// // Create a Unix-socket client node
/// let node = Node::asnc::<V2>()
///         /* define other node parameters */
/// #       .system_id(1)
/// #       .component_id(1)
///         .connection(
///             SockClient::new(path)    // Configure socket server connection
///                 .unwrap()
///         ).build().await.unwrap();
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct SockClient {
    pub(crate) path: PathBuf,
    pub(crate) info: ConnectionInfo,
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

        let info = ConnectionInfo::new(ConnectionDetails::SockClient { path: path.clone() });
        Ok(Self { path, info })
    }
}

impl ConnectionConf for SockClient {
    fn info(&self) -> &ConnectionInfo {
        &self.info
    }
}
