use async_trait::async_trait;
use std::path::{Path, PathBuf};

use tokio::fs::File;
use tokio::io::BufWriter;

use crate::asnc::io::{Connection, ConnectionBuilder};
use crate::asnc::utils::BusyReader;
use crate::core::io::{ChannelInfo, ConnectionInfo};
use crate::core::utils::SharedCloser;

use crate::prelude::*;

/// <sup>[`async`](crate::asnc)</sup>
/// Writes binary stream to a file.
///
/// Nodes built with [`FileWriter`] can't perform read operations.
///
/// # Usage
///
/// Create a node that writes to a file:
///
/// ```no_run
/// use maviola::prelude::*;
/// use maviola::sync::io::FileWriter;
///
/// let path = "/tmp/maviola.bin";
///
/// // Create a node that writes binary output to a file
/// let node = Node::builder()
///         /* define other node parameters */
/// #       .version(V2)
/// #       .system_id(1)
/// #       .component_id(1)
///         .connection(
///             FileWriter::new(path)    // Configure file reader connection
///                 .unwrap()
///         ).build().unwrap();
/// ```
#[derive(Clone, Debug)]
pub struct FileWriter {
    path: PathBuf,
    info: ConnectionInfo,
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

#[async_trait]
impl<V: MaybeVersioned + 'static> ConnectionBuilder<V> for FileWriter {
    fn info(&self) -> &ConnectionInfo {
        &self.info
    }

    async fn build(&self) -> Result<Connection<V>> {
        let path = self.path.clone();
        let file = File::create(path.as_path()).await?;

        let writer = BufWriter::new(file);
        let reader = BusyReader;

        let conn_state = SharedCloser::new();
        let (connection, chan_factory) = Connection::new(self.info.clone(), conn_state);

        let channel = chan_factory.build(ChannelInfo::FileWriter { path }, reader, writer);
        channel.spawn().await.discard();

        Ok(connection)
    }
}
