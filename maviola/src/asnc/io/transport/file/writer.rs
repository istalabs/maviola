use async_trait::async_trait;
use tokio::fs::File;
use tokio::io::BufWriter;

use crate::asnc::io::{Connection, ConnectionBuilder};
use crate::asnc::utils::BusyReader;
use crate::core::io::ChannelInfo;
use crate::core::utils::SharedCloser;

use crate::prelude::*;

#[async_trait]
impl<V: MaybeVersioned + 'static> ConnectionBuilder<V> for FileWriter {
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
