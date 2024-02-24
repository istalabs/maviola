//! # 🔒 Build extensions for MAVLink bode

use crate::core::marker::{
    HasComponentId, HasSystemId, Identified, MaybeComponentId, MaybeIdentified, MaybeSystemId,
    NoComponentId, NoConnConf, NoSystemId, Unidentified,
};
use crate::core::{Node, NodeBuilder, NodeConf};
use crate::sync::conn::ConnectionBuilder;
use crate::sync::marker::ConnConf;
use crate::sync::node::SyncApi;

use crate::prelude::*;

impl<S: MaybeSystemId, C: MaybeComponentId, D: Dialect, V: MaybeVersioned>
    NodeBuilder<S, C, D, V, NoConnConf>
{
    /// <sup>[`sync`](crate::sync)</sup>
    /// Set synchronous [`NodeConf::connection`].
    pub fn connection(
        self,
        conn_conf: impl ConnectionBuilder<V> + 'static,
    ) -> NodeBuilder<S, C, D, V, ConnConf<V>> {
        NodeBuilder {
            system_id: self.system_id,
            component_id: self.component_id,
            version: self.version,
            conn_conf: ConnConf(Box::new(conn_conf)),
            heartbeat_timeout: self.heartbeat_timeout,
            heartbeat_interval: self.heartbeat_interval,
            _dialect: self._dialect,
        }
    }
}

impl<I: MaybeIdentified, D: Dialect, V: MaybeVersioned> NodeConf<I, D, V, ConnConf<V>> {
    /// <sup>[`sync`](crate::sync)</sup>
    /// Synchronous connection configuration.
    pub fn connection(&self) -> &dyn ConnectionBuilder<V> {
        self.connection_conf.0.as_ref()
    }
}

impl<I: MaybeIdentified, D: Dialect, V: MaybeVersioned> NodeConf<I, D, V, ConnConf<V>> {
    /// <sup>[`sync`](crate::sync)</sup>
    /// Creates a [`Node`] initialised with current configuration.
    pub fn build(self) -> Result<Node<I, D, V, SyncApi<V>>> {
        Node::try_from_conf(self)
    }
}

impl<D: Dialect, V: MaybeVersioned + 'static>
    NodeBuilder<NoSystemId, NoComponentId, D, V, ConnConf<V>>
{
    /// Creates an unidentified [`Node`] with synchronous API.
    pub fn build(self) -> Result<Node<Unidentified, D, V, SyncApi<V>>> {
        Node::try_from_conf(self.conf())
    }
}

impl<D: Dialect, V: MaybeVersioned + 'static>
    NodeBuilder<HasSystemId, HasComponentId, D, V, ConnConf<V>>
{
    /// Creates an identified [`Node`] with synchronous API.
    pub fn build(self) -> Result<Node<Identified, D, V, SyncApi<V>>> {
        Node::try_from_conf(self.conf())
    }
}
