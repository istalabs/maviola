use async_trait::async_trait;
use std::marker::PhantomData;

use crate::asnc::io::{Connection, ConnectionBuilder, ConnectionHandler};
use crate::asnc::marker::AsyncConnConf;
use crate::asnc::network::conn_handler::NetworkConnectionHandler;
use crate::core::utils::Closer;

use crate::prelude::*;

#[async_trait]
impl<V: MaybeVersioned> ConnectionBuilder<V> for Network<V, AsyncConnConf<V>> {
    async fn build(&self) -> Result<(Connection<V>, ConnectionHandler)> {
        let state = Closer::new();

        let (conn, chan_factory) = Connection::new(self.info.clone(), state.to_shared());

        let conn_handler = NetworkConnectionHandler::new(state, self, chan_factory).await?;
        let handler = ConnectionHandler::spawn(async move { conn_handler.handle().await });

        Ok((conn, handler))
    }

    fn to_conf(&self) -> AsyncConnConf<V> {
        AsyncConnConf::new(Network {
            info: self.info.clone(),
            nodes: self.nodes.clone(),
            retry: self.retry,
            _version: PhantomData,
        })
    }
}
