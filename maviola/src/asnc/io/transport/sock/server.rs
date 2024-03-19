use std::path::PathBuf;

use async_trait::async_trait;
use tokio::net::{UnixListener, UnixStream};

use crate::asnc::io::{Connection, ConnectionBuilder, ConnectionHandler};
use crate::asnc::marker::AsyncConnConf;
use crate::core::consts::SERVER_HANG_UP_TIMEOUT;
use crate::core::io::{ChannelDetails, ConnectionConf, ConnectionInfo};
use crate::core::utils::{Closable, Closer};

use crate::prelude::*;

#[async_trait]
impl<V: MaybeVersioned> ConnectionBuilder<V> for SockServer {
    async fn build(&self) -> Result<(Connection<V>, ConnectionHandler)> {
        let path = self.path.clone();
        let listener = UnixListener::bind(self.path.as_path())?;

        let conn_state = Closer::new();
        let (connection, chan_factory) = Connection::new(self.info.clone(), conn_state.to_shared());

        let info = self.info().clone();

        let handler = ConnectionHandler::spawn(async move {
            on_close_handler(conn_state.to_closable(), path.clone(), info.clone());

            while !conn_state.is_closed() {
                let (stream, _) = listener.accept().await?;

                let (reader, writer) = stream.into_split();

                let chan_info =
                    info.make_channel_info(ChannelDetails::SockServer { path: path.clone() });
                let channel = chan_factory.build(chan_info, reader, writer);
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

fn on_close_handler(state: Closable, path: PathBuf, info: ConnectionInfo) {
    tokio::spawn(async move {
        while !state.is_closed() {
            tokio::time::sleep(SERVER_HANG_UP_TIMEOUT).await;
        }

        log::debug!("[{info:?}] spawn wake-up connection to close server listening loop");
        _ = UnixStream::connect(path.as_path()).await;
    });
}
