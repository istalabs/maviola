use async_trait::async_trait;
use std::path::{Path, PathBuf};

use tokio::fs::File;
use tokio::io::BufReader;

use crate::asnc::io::{Connection, ConnectionBuilder};
use crate::asnc::utils::BusyWriter;
use crate::core::io::{ChannelInfo, ConnectionInfo};
use crate::core::utils::SharedCloser;

use crate::prelude::*;

/// <sup>[`async`](crate::asnc)</sup>
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

#[async_trait]
impl<V: MaybeVersioned + 'static> ConnectionBuilder<V> for FileReader {
    fn info(&self) -> &ConnectionInfo {
        &self.info
    }

    async fn build(&self) -> Result<Connection<V>> {
        let path = self.path.clone();
        let file = File::open(path.as_path()).await?;

        let writer = BusyWriter;
        let reader = BufReader::new(file);

        let conn_state = SharedCloser::new();
        let (connection, chan_factory) = Connection::new(self.info.clone(), conn_state);

        let channel = chan_factory.build(ChannelInfo::FileReader { path }, reader, writer);
        channel.spawn().await.discard();

        Ok(connection)
    }
}
