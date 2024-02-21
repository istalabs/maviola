use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use mavio::protocol::MaybeVersioned;

use crate::io::sync::connection::{ConnectionBuilder, ConnectionConf};
use crate::io::sync::utils::BusyWriter;
use crate::io::{Connection, ConnectionInfo, PeerConnectionInfo};
use crate::utils::SharedCloser;

use crate::prelude::*;

/// Reads binary stream from existing file.
///
/// Nodes built with [`FileReaderConf`] can't perform write actions.
///
/// # Usage
///
/// Create a node that reads from a file:
///
/// ```no_run
/// # #[cfg(feature = "sync")]
/// # {
/// # use maviola::protocol::V2;
/// use maviola::{Event, Node, NodeConf, FileReaderConf};
/// # use maviola::dialects::minimal;
///
/// let path = "/tmp/maviola.bin";
///
/// // Create a node that reads binary input from a file
/// let node = Node::try_from(
///     NodeConf::builder()
///         /* define other node parameters */
/// #         .version(V2)
/// #         .system_id(1)
/// #         .component_id(1)
/// #         .dialect(minimal::dialect())
///         .connection(
///             FileReaderConf::new(path)    // Configure file reader connection
///                 .unwrap()
///         )
///         .build()
/// ).unwrap();
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct FileReaderConf {
    path: PathBuf,
    info: ConnectionInfo,
}

impl FileReaderConf {
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

impl<V: MaybeVersioned + 'static> ConnectionBuilder<V> for FileReaderConf {
    fn build(&self) -> Result<Connection<V>> {
        let path = self.path.clone();
        let file = File::open(path.as_path())?;

        let writer = BusyWriter;
        let reader = BufReader::new(file);

        let conn_state = SharedCloser::new();
        let (connection, peer_builder) = Connection::new(self.info.clone(), conn_state);

        let peer_connection =
            peer_builder.build(PeerConnectionInfo::FileReader { path }, reader, writer);
        peer_connection.spawn().as_closable();

        Ok(connection)
    }
}

impl<V: MaybeVersioned + 'static> ConnectionConf<V> for FileReaderConf {
    fn info(&self) -> &ConnectionInfo {
        &self.info
    }
}
