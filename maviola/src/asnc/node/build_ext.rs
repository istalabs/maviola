//! # 🔒 Build extensions for asynchronous MAVLink node

use std::marker::PhantomData;
use std::sync::Arc;

use crate::asnc::io::ConnectionBuilder;
use crate::asnc::marker::AsyncConnConf;
use crate::asnc::node::{AsyncApi, EdgeNode, ProxyNode};
use crate::core::marker::{
    Edge, HasComponentId, HasSystemId, MaybeComponentId, MaybeConnConf, MaybeSystemId, NodeKind,
    Unset,
};
use crate::core::node::{Node, NodeBuilder, NodeConf};
use crate::core::utils::Guarded;

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

impl<V: MaybeVersioned> NodeBuilder<HasSystemId, HasComponentId, V, Unset, AsyncApi<V>> {
    /// <sup>[`async`](crate::asnc)</sup>
    /// Creates an asynchronous [`EdgeNode`] from an existing [`ProxyNode`] reusing its connection.
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
    /// # #[tokio::main] async fn main() {
    /// use maviola::prelude::*;
    /// use maviola::asnc::prelude::*;
    ///
    /// let proxy_node = Node::asnc::<V2>()
    ///     .connection(TcpServer::new("127.0.0.1:5600").unwrap())
    ///     .build().await.unwrap();
    ///
    /// // As you can see, Rust may infer a proper
    /// // MAVLink version for us
    /// let mut edge_node = Node::asnc()
    ///     .id(MavLinkId::new(1, 17))
    ///     /* other node settings that do not include connection */
    ///     .build_from(&proxy_node);
    ///
    /// // Activate the edge node to send heartbeats
    /// edge_node.activate().await.unwrap();
    /// # }
    /// ```
    ///
    /// Yet, it is not possible to build a dependent node when connection is already set:
    ///
    /// ```rust,compile_fail
    /// # #[tokio::main] async fn main() {
    /// # use maviola::prelude::*;
    /// # use maviola::asnc::prelude::*;
    /// #
    /// let proxy_node = Node::asnc::<V2>()
    ///     .connection(TcpServer::new("127.0.0.1:5600").unwrap())
    ///     .build().await.unwrap();
    ///
    /// let mut edge_node = Node::sync()
    ///     .id(MavLinkId::new(1, 17))
    ///     .connection(TcpClient::new("127.0.0.1:5600").unwrap())
    ///     /* compilation error */
    ///     .build_from(&proxy_node);
    /// # }
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
            api: AsyncApi::new(connection, processor.clone()),
            state: Default::default(),
            is_active: Guarded::from(node.api.share_state()),
            heartbeat_timeout: self.heartbeat_timeout,
            heartbeat_interval: self.heartbeat_interval,
            processor: processor.clone(),
            _version: node._version,
        }
    }
}
