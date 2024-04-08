//! # 🔒 Build extensions for synchronous MAVLink node

use std::marker::PhantomData;
use std::sync::Arc;

use crate::core::marker::{
    Edge, HasComponentId, HasSystemId, MaybeComponentId, MaybeConnConf, MaybeSystemId, NodeKind,
    Unset,
};
use crate::core::node::{Node, NodeBuilder, NodeConf};
use crate::core::utils::Guarded;
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

impl<V: MaybeVersioned> NodeBuilder<HasSystemId, HasComponentId, V, Unset, SyncApi<V>> {
    /// <sup>[`sync`](crate::sync)</sup>
    /// Creates a synchronous [`EdgeNode`] from an existing [`ProxyNode`] reusing its connection.
    ///
    /// The new "dependant" edge nodes can be created only from a [`ProxyNode`] which means that
    /// it is impossible to create nodes of nodes and so on. The common use case is when one need
    /// several nodes per connection representing different MAVLink components.
    ///
    /// The new edge node will inherit known dialects and frame processing settings from a "parent"
    /// node. If [`signer`] and [`compat`] are not set explicitly, then they will be inherited as
    /// well. All [custom processors](crate::docs::c3__custom_processing) from the "parent" node
    /// will be added to the new one.
    ///
    /// **⚠** The [`heartbeat_timeout`] setting of a "parent" [`ProxyNode`] node will be ignored!
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use maviola::prelude::*;
    /// use maviola::sync::prelude::*;
    ///
    /// let proxy_node = Node::sync::<V2>()
    ///     .connection(TcpServer::new("127.0.0.1:5600").unwrap())
    ///     .build().unwrap();
    ///
    /// // As you can see, Rust may infer a proper
    /// // MAVLink version for us
    /// let mut edge_node = Node::sync()
    ///     .id(MavLinkId::new(1, 17))
    ///     /* other node settings that do not include connection */
    ///     .build_from(&proxy_node);
    ///
    /// // Activate the edge node to send heartbeats
    /// edge_node.activate().unwrap();
    /// ```
    ///
    /// Yet, it is not possible to build a dependent node when connection is already set:
    ///
    /// ```rust,compile_fail
    /// # use maviola::prelude::*;
    /// # use maviola::sync::prelude::*;
    /// #
    /// let proxy_node = Node::sync::<V2>()
    ///     .connection(TcpServer::new("127.0.0.1:5600").unwrap())
    ///     .build().unwrap();
    ///
    /// let mut edge_node = Node::sync()
    ///     .id(MavLinkId::new(1, 17))
    ///     .connection(TcpClient::new("127.0.0.1:5600").unwrap())
    ///     /* compilation error */
    ///     .build_from(&proxy_node);
    /// ```
    ///
    /// [`signer`]: Self::signer
    /// [`compat`]: Self::compat
    /// [`heartbeat_timeout`]: Self::heartbeat_timeout
    pub fn build_from(self, node: &ProxyNode<V>) -> EdgeNode<V> {
        let processor = Arc::new(self.reuse_processor(node.processor.as_ref()));
        let connection = node.api.connection().reuse();

        Node {
            kind: Edge::new(Endpoint::new(MavLinkId::new(
                self.system_id.0,
                self.component_id.0,
            ))),
            api: SyncApi::new(connection, processor.clone()),
            state: Default::default(),
            is_active: Guarded::from(node.api.share_state()),
            heartbeat_timeout: self.heartbeat_timeout,
            heartbeat_interval: self.heartbeat_interval,
            processor: processor.clone(),
            _version: node._version,
        }
    }
}
