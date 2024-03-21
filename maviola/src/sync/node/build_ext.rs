//! # 🔒 Build extensions for synchronous MAVLink node

use std::marker::PhantomData;

use crate::core::marker::{
    HasComponentId, HasSystemId, MaybeComponentId, MaybeConnConf, MaybeSystemId, NodeKind, Unset,
};
use crate::core::node::{Node, NodeBuilder, NodeConf};
use crate::sync::io::ConnectionBuilder;
use crate::sync::marker::ConnConf;
use crate::sync::node::{EdgeNode, ProxyNode, SyncApi};

use crate::prelude::*;

impl NodeBuilder<Unset, Unset, Versionless, Unset, Unset> {
    /// <sup>[`sync`](crate::sync)</sup>
    /// Create node builder in synchronous mode.
    pub fn synchronous() -> NodeBuilder<Unset, Unset, Versionless, Unset, SyncApi<Versionless>> {
        NodeBuilder::default().sync()
    }
}

impl<S: MaybeSystemId, C: MaybeComponentId, V: MaybeVersioned> NodeBuilder<S, C, V, Unset, Unset> {
    /// <sup>[`sync`](crate::sync)</sup>
    /// Turn node builder into synchronous mode.
    pub fn sync(self) -> NodeBuilder<S, C, V, Unset, SyncApi<V>> {
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
    NodeBuilder<S, C, V, CC, SyncApi<V>>
{
    /// <sup>[`sync`](crate::sync)</sup>
    /// Set [`NodeConf::version`].
    ///
    /// This method is available only when API is set to sync mode via [`NodeBuilder::sync`] or
    /// builder was created as [`NodeBuilder::synchronous`].
    pub fn version<Version: MaybeVersioned>(
        self,
    ) -> NodeBuilder<S, C, Version, CC, SyncApi<Version>> {
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
    NodeBuilder<S, C, V, Unset, SyncApi<V>>
{
    /// <sup>[`sync`](crate::sync)</sup>
    /// Set synchronous [`NodeConf::connection`].
    pub fn connection(
        self,
        conn_conf: impl ConnectionBuilder<V> + 'static,
    ) -> NodeBuilder<S, C, V, ConnConf<V>, SyncApi<V>> {
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
            _api: self._api,
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

impl<V: MaybeVersioned> NodeBuilder<Unset, Unset, V, ConnConf<V>, SyncApi<V>> {
    /// <sup>[`sync`](crate::sync)</sup>
    /// Creates a [`ProxyNode`] with synchronous API.
    pub fn build(self) -> Result<ProxyNode<V>> {
        Node::try_from_conf(self.conf())
    }
}

impl<V: MaybeVersioned> NodeBuilder<HasSystemId, HasComponentId, V, ConnConf<V>, SyncApi<V>> {
    /// <sup>[`sync`](crate::sync)</sup>
    /// Creates an [`EdgeNode`] with synchronous API.
    pub fn build(self) -> Result<EdgeNode<V>> {
        Node::try_from_conf(self.conf())
    }
}
