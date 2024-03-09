//! # 🔒 Build extensions for asynchronous MAVLink node

use crate::asnc::io::ConnectionBuilder;
use crate::asnc::marker::AsyncConnConf;
use crate::asnc::node::{AsyncApi, EdgeNode, ProxyNode};
use crate::core::marker::{
    HasComponentId, HasSystemId, MaybeComponentId, MaybeSystemId, NodeKind, Unset,
};
use crate::core::node::{Node, NodeBuilder, NodeConf};

use crate::prelude::*;

impl<S: MaybeSystemId, C: MaybeComponentId, V: MaybeVersioned + 'static>
    NodeBuilder<S, C, V, Unset>
{
    /// <sup>[`async`](crate::asnc)</sup>
    /// Set asynchronous [`NodeConf::connection`].
    pub fn async_connection(
        self,
        conn_conf: impl ConnectionBuilder<V> + 'static,
    ) -> NodeBuilder<S, C, V, AsyncConnConf<V>> {
        NodeBuilder {
            system_id: self.system_id,
            component_id: self.component_id,
            conn_conf: AsyncConnConf::new(conn_conf),
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

impl<K: NodeKind, V: MaybeVersioned + 'static> NodeConf<K, V, AsyncConnConf<V>> {
    /// <sup>[`async`](crate::asnc)</sup>
    /// Synchronous connection configuration.
    pub fn connection(&self) -> &dyn ConnectionBuilder<V> {
        self.connection_conf.connection()
    }
}

impl<K: NodeKind, V: MaybeVersioned> NodeConf<K, V, AsyncConnConf<V>> {
    /// <sup>[`async`](crate::asnc)</sup>
    /// Creates a [`Node`] initialised with current configuration.
    pub async fn build(self) -> Result<Node<K, V, AsyncApi<V>>> {
        Node::try_from_async_conf(self).await
    }
}

impl<V: MaybeVersioned + 'static> NodeBuilder<Unset, Unset, V, AsyncConnConf<V>> {
    /// <sup>[`async`](crate::asnc)</sup>
    /// Creates an [`ProxyNode`] with synchronous API.
    pub async fn build(self) -> Result<ProxyNode<V>> {
        Node::try_from_async_conf(self.conf()).await
    }
}

impl<V: MaybeVersioned + 'static> NodeBuilder<HasSystemId, HasComponentId, V, AsyncConnConf<V>> {
    /// <sup>[`async`](crate::asnc)</sup>
    /// Creates an [`EdgeNode`] with synchronous API.
    pub async fn build(self) -> Result<EdgeNode<V>> {
        Node::try_from_async_conf(self.conf()).await
    }
}
