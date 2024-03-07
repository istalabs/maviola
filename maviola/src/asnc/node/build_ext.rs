//! # 🔒 Build extensions for asynchronous MAVLink node

use crate::asnc::io::ConnectionBuilder;
use crate::asnc::marker::AsyncConnConf;
use crate::asnc::node::{AsyncApi, EdgeNode, ProxyNode};
use crate::core::marker::{
    HasComponentId, HasSystemId, MaybeComponentId, MaybeSystemId, NoComponentId, NoConnConf,
    NoSystemId, NodeKind,
};
use crate::core::node::{Node, NodeBuilder, NodeConf};

use crate::prelude::*;

impl<S: MaybeSystemId, C: MaybeComponentId, D: Dialect, V: MaybeVersioned + 'static>
    NodeBuilder<S, C, D, V, NoConnConf>
{
    /// <sup>[`async`](crate::asnc)</sup>
    /// Set asynchronous [`NodeConf::connection`].
    pub fn async_connection(
        self,
        conn_conf: impl ConnectionBuilder<V> + 'static,
    ) -> NodeBuilder<S, C, D, V, AsyncConnConf<V>> {
        NodeBuilder {
            system_id: self.system_id,
            component_id: self.component_id,
            version: self.version,
            conn_conf: AsyncConnConf(Box::new(conn_conf)),
            signer: self.signer,
            heartbeat_timeout: self.heartbeat_timeout,
            heartbeat_interval: self.heartbeat_interval,
            _dialect: self._dialect,
        }
    }
}

impl<K: NodeKind, D: Dialect, V: MaybeVersioned> NodeConf<K, D, V, AsyncConnConf<V>> {
    /// <sup>[`async`](crate::asnc)</sup>
    /// Synchronous connection configuration.
    pub fn connection(&self) -> &dyn ConnectionBuilder<V> {
        self.connection_conf.0.as_ref()
    }
}

impl<K: NodeKind, D: Dialect, V: MaybeVersioned> NodeConf<K, D, V, AsyncConnConf<V>> {
    /// <sup>[`async`](crate::asnc)</sup>
    /// Creates a [`Node`] initialised with current configuration.
    pub async fn build(self) -> Result<Node<K, D, V, AsyncApi<V>>> {
        Node::try_from_async_conf(self).await
    }
}

impl<D: Dialect, V: MaybeVersioned + 'static>
    NodeBuilder<NoSystemId, NoComponentId, D, V, AsyncConnConf<V>>
{
    /// <sup>[`async`](crate::asnc)</sup>
    /// Creates an [`ProxyNode`] with synchronous API.
    pub async fn build(self) -> Result<ProxyNode<D, V>> {
        Node::try_from_async_conf(self.conf()).await
    }
}

impl<D: Dialect, V: MaybeVersioned + 'static>
    NodeBuilder<HasSystemId, HasComponentId, D, V, AsyncConnConf<V>>
{
    /// <sup>[`async`](crate::asnc)</sup>
    /// Creates an [`EdgeNode`] with synchronous API.
    pub async fn build(self) -> Result<EdgeNode<D, V>> {
        Node::try_from_async_conf(self.conf()).await
    }
}
