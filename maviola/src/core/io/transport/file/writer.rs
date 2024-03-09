use std::path::{Path, PathBuf};

use crate::core::io::{ConnectionConf, ConnectionInfo};

use crate::prelude::*;

/// Writes binary stream to a file.
///
/// Nodes built with [`FileWriter`] can't perform read operations.
///
/// # Usage
///
/// Create a synchronous node that writes to a file:
///
/// ```rust,no_run
/// use maviola::prelude::*;
///
/// let path = "/tmp/maviola.bin";
///
/// // Create a node that writes binary output to a file
/// let node = Node::builder()
///         /* define other node parameters */
/// #       .version::<V2>()
/// #       .system_id(1)
/// #       .component_id(1)
///         .connection(
///             FileWriter::new(path)    // Configure file reader connection
///                 .unwrap()
///         ).build().unwrap();
/// ```
///
/// # Usage
///
/// Create an asynchronous node that writes to a file:
///
/// ```rust,no_run
/// # #[tokio::main] async fn main() {
/// use maviola::prelude::*;
///
/// let path = "/tmp/maviola.bin";
///
/// // Create a node that writes binary output to a file
/// let node = Node::builder()
///         /* define other node parameters */
/// #       .version::<V2>()
/// #       .system_id(1)
/// #       .component_id(1)
///         .async_connection(
///             FileWriter::new(path)    // Configure file reader connection
///                 .unwrap()
///         ).build().await.unwrap();
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct FileWriter {
    pub(crate) path: PathBuf,
    pub(crate) info: ConnectionInfo,
}

impl FileWriter {
    /// Instantiates a file writer configuration.
    ///
    /// Accepts as `path` anything that can be converted to [`PathBuf`], validates that file does
    /// not exist.
    pub fn new(path: impl Into<PathBuf>) -> Result<Self> {
        let path: PathBuf = path.into();

        if Path::exists(path.as_path()) {
            return Err(Error::from(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("file already exists: {path:?}"),
            )));
        }

        let info = ConnectionInfo::FileWriter { path: path.clone() };
        Ok(Self { path, info })
    }
}

impl ConnectionConf for FileWriter {
    fn info(&self) -> &ConnectionInfo {
        &self.info
    }
}
