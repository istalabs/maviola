//! # 🔒 Build extensions for synchronous MAVLink node

use crate::core::marker::{
    HasComponentId, HasSystemId, MaybeComponentId, MaybeSystemId, NoComponentId, NoConnConf,
    NoSystemId, NodeKind,
};
use crate::core::node::{Node, NodeBuilder, NodeConf};
use crate::sync::io::ConnectionBuilder;
use crate::sync::marker::ConnConf;
use crate::sync::node::{EdgeNode, ProxyNode, SyncApi};

use crate::prelude::*;

impl<S: MaybeSystemId, C: MaybeComponentId, D: Dialect, V: MaybeVersioned + 'static>
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

impl<K: NodeKind, D: Dialect, V: MaybeVersioned> NodeConf<K, D, V, ConnConf<V>> {
    /// <sup>[`sync`](crate::sync)</sup>
    /// Synchronous connection configuration.
    pub fn connection(&self) -> &dyn ConnectionBuilder<V> {
        self.connection_conf.0.as_ref()
    }
}

impl<K: NodeKind, D: Dialect, V: MaybeVersioned> NodeConf<K, D, V, ConnConf<V>> {
    /// <sup>[`sync`](crate::sync)</sup>
    /// Creates a [`Node`] initialised with current configuration.
    pub fn build(self) -> Result<Node<K, D, V, SyncApi<V>>> {
        Node::try_from_conf(self)
    }
}

impl<D: Dialect, V: MaybeVersioned + 'static>
    NodeBuilder<NoSystemId, NoComponentId, D, V, ConnConf<V>>
{
    /// <sup>[`sync`](crate::sync)</sup>
    /// Creates a [`ProxyNode`] with synchronous API.
    pub fn build(self) -> Result<ProxyNode<D, V>> {
        Node::try_from_conf(self.conf())
    }
}

impl<D: Dialect, V: MaybeVersioned + 'static>
    NodeBuilder<HasSystemId, HasComponentId, D, V, ConnConf<V>>
{
    /// <sup>[`sync`](crate::sync)</sup>
    /// Creates an [`EdgeNode`] with synchronous API.
    pub fn build(self) -> Result<EdgeNode<D, V>> {
        Node::try_from_conf(self.conf())
    }
}
