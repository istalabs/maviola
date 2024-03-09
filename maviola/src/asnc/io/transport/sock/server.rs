use async_trait::async_trait;
use tokio::net::UnixListener;

use crate::asnc::io::{Connection, ConnectionBuilder, ConnectionHandler};
use crate::asnc::marker::AsyncConnConf;
use crate::core::io::ChannelInfo;
use crate::core::utils::Closer;

use crate::prelude::*;

#[async_trait]
impl<V: MaybeVersioned + 'static> ConnectionBuilder<V> for SockServer {
    async fn build(&self) -> Result<(Connection<V>, ConnectionHandler)> {
        let path = self.path.clone();
        let listener = UnixListener::bind(self.path.as_path())?;

        let conn_state = Closer::new();
        let (connection, chan_factory) = Connection::new(self.info.clone(), conn_state.to_shared());

        let handler = ConnectionHandler::spawn(async move {
            while !conn_state.is_closed() {
                let (stream, _) = listener.accept().await?;

                let (reader, writer) = stream.into_split();

                let channel = chan_factory.build(
                    ChannelInfo::SockServer { path: path.clone() },
                    reader,
                    writer,
                );
                channel.spawn().await.discard();
            }

            Ok(())
        });

        Ok((connection, handler))
    }

    fn to_conf(&self) -> AsyncConnConf<V> {
        AsyncConnConf::new(self.clone())
    }
}
