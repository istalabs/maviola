//! # 🔒 Build extensions for synchronous MAVLink node

use crate::core::marker::{
    HasComponentId, HasSystemId, MaybeComponentId, MaybeSystemId, NodeKind, Unset,
};
use crate::core::node::{Node, NodeBuilder, NodeConf};
use crate::sync::io::ConnectionBuilder;
use crate::sync::marker::ConnConf;
use crate::sync::node::{EdgeNode, ProxyNode, SyncApi};

use crate::prelude::*;

impl<S: MaybeSystemId, C: MaybeComponentId, V: MaybeVersioned + 'static>
    NodeBuilder<S, C, V, Unset>
{
    /// <sup>[`sync`](crate::sync)</sup>
    /// Set synchronous [`NodeConf::connection`].
    pub fn connection(
        self,
        conn_conf: impl ConnectionBuilder<V> + 'static,
    ) -> NodeBuilder<S, C, V, ConnConf<V>> {
        NodeBuilder {
            system_id: self.system_id,
            component_id: self.component_id,
            conn_conf: ConnConf::new(conn_conf),
            heartbeat_timeout: self.heartbeat_timeout,
            heartbeat_interval: self.heartbeat_interval,
            dialects: self.dialects,
            signer: self.signer,
            compat: self.compat,
            processors: self.processors,
            _version: self._version,
        }
    }
}

impl<K: NodeKind, V: MaybeVersioned> NodeConf<K, V, ConnConf<V>> {
    /// <sup>[`sync`](crate::sync)</sup>
    /// Synchronous connection configuration.
    pub fn connection(&self) -> &dyn ConnectionBuilder<V> {
        self.connection_conf.connection()
    }
}

impl<K: NodeKind, V: MaybeVersioned> NodeConf<K, V, ConnConf<V>> {
    /// <sup>[`sync`](crate::sync)</sup>
    /// Creates a [`Node`] initialised with current configuration.
    pub fn build(self) -> Result<Node<K, V, SyncApi<V>>> {
        Node::try_from_conf(self)
    }
}

impl<V: MaybeVersioned + 'static> NodeBuilder<Unset, Unset, V, ConnConf<V>> {
    /// <sup>[`sync`](crate::sync)</sup>
    /// Creates a [`ProxyNode`] with synchronous API.
    pub fn build(self) -> Result<ProxyNode<V>> {
        Node::try_from_conf(self.conf())
    }
}

impl<V: MaybeVersioned + 'static> NodeBuilder<HasSystemId, HasComponentId, V, ConnConf<V>> {
    /// <sup>[`sync`](crate::sync)</sup>
    /// Creates an [`EdgeNode`] with synchronous API.
    pub fn build(self) -> Result<EdgeNode<V>> {
        Node::try_from_conf(self.conf())
    }
}
