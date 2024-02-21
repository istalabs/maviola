use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use mavio::protocol::MaybeVersioned;

use crate::io::sync::connection::{ConnectionBuilder, ConnectionConf};
use crate::io::sync::utils::BusyReader;
use crate::io::{Connection, ConnectionInfo, PeerConnectionInfo};
use crate::utils::SharedCloser;

use crate::prelude::*;

/// Writes binary stream to a file.
///
/// Nodes built with [`FileWriterConf`] can't perform read operations.
///
/// # Usage
///
/// Create a node that writes to a file:
///
/// ```no_run
/// # #[cfg(feature = "sync")]
/// # {
/// # use maviola::protocol::V2;
/// use maviola::{Event, Node, NodeConf, FileWriterConf};
/// # use maviola::dialects::minimal;
///
/// let path = "/tmp/maviola.bin";
///
/// // Create a node that writes binary output to a file
/// let node = Node::try_from(
///     NodeConf::builder()
///         /* define other node parameters */
/// #         .version(V2)
/// #         .system_id(1)
/// #         .component_id(1)
/// #         .dialect(minimal::dialect())
///         .connection(
///             FileWriterConf::new(path)    // Configure file reader connection
///                 .unwrap()
///         )
///         .build()
/// ).unwrap();
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct FileWriterConf {
    path: PathBuf,
    info: ConnectionInfo,
}

impl FileWriterConf {
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

impl<V: MaybeVersioned + 'static> ConnectionBuilder<V> for FileWriterConf {
    fn build(&self) -> Result<Connection<V>> {
        let path = self.path.clone();
        let file = File::create(path.as_path())?;

        let writer = BufWriter::new(file);
        let reader = BusyReader;

        let conn_state = SharedCloser::new();
        let (connection, peer_builder) = Connection::new(self.info.clone(), conn_state);

        let peer_connection =
            peer_builder.build(PeerConnectionInfo::FileWriter { path }, reader, writer);
        peer_connection.spawn().as_closable();

        Ok(connection)
    }
}

impl<V: MaybeVersioned + 'static> ConnectionConf<V> for FileWriterConf {
    fn info(&self) -> &ConnectionInfo {
        &self.info
    }
}
