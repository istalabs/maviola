use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use crate::core::io::{ChannelInfo, ConnectionInfo};
use crate::core::utils::SharedCloser;
use crate::sync::io::{Connection, ConnectionBuilder};
use crate::sync::utils::BusyWriter;

use crate::prelude::*;

/// <sup>[`sync`](crate::sync)</sup>
/// Reads binary stream from existing file.
///
/// Nodes built with [`FileReader`] can't perform write actions.
///
/// # Usage
///
/// Create a node that reads from a file:
///
/// ```no_run
/// use maviola::prelude::*;
/// use maviola::sync::io::FileReader;
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
#[derive(Clone, Debug)]
pub struct FileReader {
    path: PathBuf,
    info: ConnectionInfo,
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

impl<V: MaybeVersioned + 'static> ConnectionBuilder<V> for FileReader {
    fn info(&self) -> &ConnectionInfo {
        &self.info
    }

    fn build(&self) -> Result<Connection<V>> {
        let path = self.path.clone();
        let file = File::open(path.as_path())?;

        let writer = BusyWriter;
        let reader = BufReader::new(file);

        let conn_state = SharedCloser::new();
        let (connection, peer_builder) = Connection::new(self.info.clone(), conn_state);

        let peer_connection = peer_builder.build(ChannelInfo::FileReader { path }, reader, writer);
        peer_connection.spawn().to_closable();

        Ok(connection)
    }
}