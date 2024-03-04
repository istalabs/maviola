use async_trait::async_trait;
use tokio::fs::File;
use tokio::io::BufReader;

use crate::asnc::io::{Connection, ConnectionBuilder};
use crate::asnc::utils::BusyWriter;
use crate::core::io::ChannelInfo;
use crate::core::utils::SharedCloser;

use crate::prelude::*;

#[async_trait]
impl<V: MaybeVersioned + 'static> ConnectionBuilder<V> for FileReader {
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
