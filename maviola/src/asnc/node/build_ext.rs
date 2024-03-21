//! # 🔒 Build extensions for asynchronous MAVLink node

use std::marker::PhantomData;

use crate::asnc::io::ConnectionBuilder;
use crate::asnc::marker::AsyncConnConf;
use crate::asnc::node::{AsyncApi, EdgeNode, ProxyNode};
use crate::core::marker::{
    HasComponentId, HasSystemId, MaybeComponentId, MaybeConnConf, MaybeSystemId, NodeKind, Unset,
};
use crate::core::node::{Node, NodeBuilder, NodeConf};

use crate::prelude::*;

impl NodeBuilder<Unset, Unset, Versionless, Unset, Unset> {
    /// <sup>[`async`](crate::asnc)</sup>
    /// Create node builder in asynchronous mode.
    pub fn asynchronous() -> NodeBuilder<Unset, Unset, Versionless, Unset, AsyncApi<Versionless>> {
        NodeBuilder::default().asnc()
    }
}

impl<S: MaybeSystemId, C: MaybeComponentId, V: MaybeVersioned> NodeBuilder<S, C, V, Unset, Unset> {
    /// <sup>[`async`](crate::asnc)</sup>
    /// Turn node builder into asynchronous mode.
    pub fn asnc(self) -> NodeBuilder<S, C, V, Unset, AsyncApi<V>> {
        NodeBuilder {
            system_id: self.system_id,
            component_id: self.component_id,
            conn_conf: self.conn_conf,
            heartbeat_timeout: self.heartbeat_timeout,
            heartbeat_interval: self.heartbeat_interval,
            dialects: self.dialects,
            signer: self.signer,
            compat: self.compat,
            processors: self.processors,
            _version: self._version,
            _api: PhantomData,
        }
    }
}

impl<S: MaybeSystemId, C: MaybeComponentId, V: MaybeVersioned, CC: MaybeConnConf>
    NodeBuilder<S, C, V, CC, AsyncApi<V>>
{
    /// <sup>[`async`](crate::asnc)</sup>
    /// Set [`NodeConf::version`].
    ///
    /// This method is available only when API is set to sync mode via [`NodeBuilder::asnc`] or
    /// builder was created as [`NodeBuilder::asynchronous`].
    pub fn version<Version: MaybeVersioned>(
        self,
    ) -> NodeBuilder<S, C, Version, CC, AsyncApi<Version>> {
        NodeBuilder {
            system_id: self.system_id,
            component_id: self.component_id,
            conn_conf: self.conn_conf,
            heartbeat_timeout: self.heartbeat_timeout,
            heartbeat_interval: self.heartbeat_interval,
            dialects: self.dialects,
            signer: self.signer,
            compat: self.compat,
            processors: self.processors,
            _version: PhantomData,
            _api: PhantomData,
        }
    }
}

impl<S: MaybeSystemId, C: MaybeComponentId, V: MaybeVersioned>
    NodeBuilder<S, C, V, Unset, AsyncApi<V>>
{
    /// <sup>[`async`](crate::asnc)</sup>
    /// Set asynchronous [`NodeConf::connection`].
    pub fn connection(
        self,
        conn_conf: impl ConnectionBuilder<V> + 'static,
    ) -> NodeBuilder<S, C, V, AsyncConnConf<V>, AsyncApi<V>> {
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
            _api: self._api,
        }
    }
}

impl<K: NodeKind, V: MaybeVersioned> NodeConf<K, V, AsyncConnConf<V>> {
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

impl<V: MaybeVersioned> NodeBuilder<Unset, Unset, V, AsyncConnConf<V>, AsyncApi<V>> {
    /// <sup>[`async`](crate::asnc)</sup>
    /// Creates an [`ProxyNode`] with synchronous API.
    pub async fn build(self) -> Result<ProxyNode<V>> {
        Node::try_from_async_conf(self.conf()).await
    }
}

impl<V: MaybeVersioned> NodeBuilder<HasSystemId, HasComponentId, V, AsyncConnConf<V>, AsyncApi<V>> {
    /// <sup>[`async`](crate::asnc)</sup>
    /// Creates an [`EdgeNode`] with synchronous API.
    pub async fn build(self) -> Result<EdgeNode<V>> {
        Node::try_from_async_conf(self.conf()).await
    }
}
