use std::path::{Path, PathBuf};

use crate::core::io::{ConnectionConf, ConnectionInfo};

use crate::prelude::*;

/// Reads binary stream from existing file.
///
/// Nodes built with [`FileReader`] can't perform write actions.
///
/// # Usage
///
/// Create a synchronous node that reads from a file:
///
/// ```rust,no_run
/// use maviola::prelude::*;
///
/// let path = "/tmp/maviola.bin";
///
/// // Create a node that reads binary input from a file
/// let node = Node::builder()
///         /* define other node parameters */
/// #       .version(V2)
/// #       .system_id(1)
/// #       .component_id(1)
///         .connection(
///             FileReader::new(path)    // Configure file reader connection
///                 .unwrap()
///         ).build().unwrap();
/// ```
///
/// Create an asynchronous node that reads from a file:
///
/// ```rust,no_run
/// # #[tokio::main] async fn main() {
/// use maviola::prelude::*;
///
/// let path = "/tmp/maviola.bin";
///
/// // Create a node that reads binary input from a file
/// let node = Node::builder()
///         /* define other node parameters */
/// #       .version(V2)
/// #       .system_id(1)
/// #       .component_id(1)
///         .async_connection(
///             FileReader::new(path)    // Configure file reader connection
///                 .unwrap()
///         ).build().await.unwrap();
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct FileReader {
    pub(crate) path: PathBuf,
    pub(crate) info: ConnectionInfo,
}

impl FileReader {
    /// Instantiates a file reader configuration.
    ///
    /// Accepts as `path` anything that can be converted to [`PathBuf`], validates that file already
    /// exist and indeed is a file.
    pub fn new(path: impl Into<PathBuf>) -> Result<Self> {
        let path: PathBuf = path.into();

        if !Path::exists(path.as_path()) {
            return Err(Error::from(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("file does not exists: {path:?}"),
            )));
        }

        if !Path::is_file(path.as_path()) {
            return Err(Error::from(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("not a file: {path:?}"),
            )));
        }

        let info = ConnectionInfo::FileReader { path: path.clone() };
        Ok(Self { path, info })
    }
}

impl ConnectionConf for FileReader {
    fn info(&self) -> &ConnectionInfo {
        &self.info
    }
}
