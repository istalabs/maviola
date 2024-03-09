use std::marker::PhantomData;

use crate::core::utils::Closer;
use crate::sync::io::{Connection, ConnectionBuilder, ConnectionHandler};
use crate::sync::marker::ConnConf;
use crate::sync::network::conn_handler::NetworkConnectionHandler;

use crate::prelude::*;

impl<V: MaybeVersioned> ConnectionBuilder<V> for Network<V, ConnConf<V>> {
    fn build(&self) -> Result<(Connection<V>, ConnectionHandler)> {
        let state = Closer::new();

        let (conn, chan_factory) = Connection::new(self.info.clone(), state.to_shared());

        let conn_handler = NetworkConnectionHandler::new(state, self, chan_factory)?;
        let handler = ConnectionHandler::spawn(move || conn_handler.handle());

        Ok((conn, handler))
    }

    fn to_conf(&self) -> ConnConf<V> {
        ConnConf::new(Network {
            info: self.info.clone(),
            nodes: self.nodes.clone(),
            retry: self.retry,
            _version: PhantomData,
        })
    }
}
