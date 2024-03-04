use std::path::{Path, PathBuf};

use crate::core::io::{ConnectionConf, ConnectionInfo};

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
/// ```no_run
/// use maviola::prelude::*;
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

        let info = ConnectionInfo::SockClient { path: path.clone() };
        Ok(Self { path, info })
    }
}

impl ConnectionConf for SockClient {
    fn info(&self) -> &ConnectionInfo {
        &self.info
    }
}
