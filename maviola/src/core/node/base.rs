use std::marker::PhantomData;
use std::time::Duration;

use crate::core::io::ConnectionInfo;
use crate::core::marker::{Edge, NoComponentId, NoConnConf, NoSystemId, NodeKind, Proxy};
use crate::core::node::api::{NoApi, NodeApi};
use crate::core::node::NodeBuilder;
use crate::core::utils::{Guarded, SharedCloser, Switch};
use crate::protocol::{
    ComponentId, Dialect, MavLinkVersion, MaybeVersioned, Message, SystemId, Versioned, Versionless,
};

use crate::prelude::*;

/// MAVLink node.
///
/// A node is a member of a MAVLink network that manages I/O connection and provides interface for
/// communicating with other MAVLink devices. [`Node`] is API-agnostic, both synchronous and
/// asynchronous API extend its functionality with specific methods and behavior relevant to the
/// underlying concurrency model. Asynchronous API is based on [Tokio](https://tokio.rs).
///
/// There are two fundamental kinds of a node defined by [`NodeKind`] generic parameter:
///
/// * Edge node ([`asnc::node::EdgeNode`](crate::asnc::node::EdgeNode) /
///   [`sync::node::EdgeNode`](crate::sync::node::EdgeNode)) is a MAVlink device with defined system
///   `ID` and component `ID`. It can send and receive MAVLink messages, emit automatic heartbeats
///   and perform other active tasks. This is the kind of node you are mostly interested in.
/// * Proxy node ([`asnc::node::ProxyNode`](crate::asnc::node::ProxyNode) /
///   [`sync::node::ProxyNode`](crate::sync::node::ProxyNode)), on the other hand, does not have a
///   specified `ID` and component `ID`. It only can receive and proxy MAVLink frames. This node
///   can't perform active tasks and is used to pass frames between different parts of a MAVLink
///   network.
///
/// # Examples
///
/// Create a synchronous TCP server node:
///
/// ```no_run
/// use maviola::prelude::*;
/// use maviola::sync::prelude::*;
///
/// let addr = "127.0.0.1:5600";
///
/// // Create a node from configuration
/// let mut node = Node::builder()
///     .version(V2)                // restrict node to MAVLink2 protocol version
///     .system_id(1)               // System `ID`
///     .component_id(1)            // Component `ID`
///     .dialect::<Minimal>()       // Dialect is set to `minimal`
///     .connection(
///         TcpServer::new(addr)    // Configure TCP server connection
///             .unwrap()
///     ).build().unwrap();
///
/// // Activate node to start sending heartbeats
/// node.activate().unwrap();
/// ```
///
/// Create an asynchronous TCP server node:
///
/// ```no_run
/// # #[tokio::main(flavor = "current_thread")] async fn main() {
/// use maviola::prelude::*;
/// use maviola::asnc::prelude::*;
///
/// let addr = "127.0.0.1:5600";
///
/// // Create a node from configuration
/// let mut node = Node::builder()
///     .version(V2)                // restrict node to MAVLink2 protocol version
///     .system_id(1)               // System `ID`
///     .component_id(1)            // Component `ID`
///     .dialect::<Minimal>()       // Dialect is set to `minimal`
///     .async_connection(
///         TcpServer::new(addr)    // Configure TCP server connection
///             .unwrap()
///     ).build().await.unwrap();
///
/// // Activate node to start sending heartbeats
/// node.activate().await.unwrap();
/// # }
/// ```
pub struct Node<K: NodeKind, D: Dialect, V: MaybeVersioned + 'static, A: NodeApi<V>> {
    pub(crate) kind: K,
    pub(crate) api: A,
    pub(crate) state: SharedCloser,
    pub(crate) is_active: Guarded<SharedCloser, Switch>,
    pub(crate) heartbeat_timeout: Duration,
    pub(crate) heartbeat_interval: Duration,
    pub(crate) _dialect: PhantomData<D>,
    pub(crate) _version: PhantomData<V>,
}

impl Node<Proxy, Minimal, Versionless, NoApi> {
    /// Instantiates an empty [`NodeBuilder`].
    pub fn builder() -> NodeBuilder<NoSystemId, NoComponentId, Minimal, Versionless, NoConnConf> {
        NodeBuilder::new()
    }
}

impl<K: NodeKind, D: Dialect, V: MaybeVersioned + 'static, A: NodeApi<V>> Node<K, D, V, A> {
    /// Information about this node's connection.
    pub fn info(&self) -> &ConnectionInfo {
        self.api.info()
    }

    /// Heartbeat timeout.
    ///
    /// For peers that overdue to send the next heartbeat within this interval will be considered
    /// inactive.
    ///
    /// Default value is [`DEFAULT_HEARTBEAT_TIMEOUT`](crate::core::consts::DEFAULT_HEARTBEAT_TIMEOUT).
    pub fn heartbeat_timeout(&self) -> Duration {
        self.heartbeat_timeout
    }

    /// Returns `true` if node is connected.
    ///
    /// All nodes are connected by default, they can become disconnected only if I/O transport
    /// failed or have been exhausted.
    pub fn is_connected(&self) -> bool {
        !self.state.is_closed()
    }

    fn close(&mut self) {
        self.state.close();

        log::debug!("{:?}: node is closed", self.info());
    }
}

impl<D: Dialect, V: Versioned + 'static, A: NodeApi<V>> Node<Edge<V>, D, V, A> {
    /// Send MAVLink message.
    ///
    /// The message will be encoded according to the node's dialect specification and MAVLink
    /// protocol version.
    ///
    /// If you want to send messages within different MAVLink protocols simultaneously, you have
    /// to construct a [`Versionless`] node and use [`Node::send_versioned`]
    pub fn send(&self, message: &impl Message) -> Result<()> {
        let frame = self.kind.endpoint.next_frame(message)?;
        self.api.send_frame(&frame)
    }

    /// Returns `true`, if node is active.
    ///
    /// All nodes are inactive by default and have to be activated using [`Node::activate`].
    ///
    /// Active nodes will send heartbeats and perform other automated operations which do not
    /// require direct initiative from the user.
    ///
    /// Inactive nodes will neither send heartbeats, nor perform other operations which are not
    /// directly requested by user. They will still receive incoming frames and emit corresponding
    /// events.
    ///
    /// Active nodes are also connected and [`Node::is_connected`] will return `true`.
    ///
    /// Node transitions into inactive state when it becomes disconnected or when
    /// [`Node::deactivate`] called.
    pub fn is_active(&self) -> bool {
        self.is_active.is()
    }

    /// Heartbeat interval.
    ///
    /// Once node is started using [`Node::activate`], it will emit heartbeats with this interval.
    ///
    /// Default value is [`DEFAULT_HEARTBEAT_INTERVAL`](crate::core::consts::DEFAULT_HEARTBEAT_INTERVAL).
    pub fn heartbeat_interval(&self) -> Duration {
        self.heartbeat_interval
    }

    /// Deactivates the node.
    ///
    /// Inactive nodes will neither send heartbeats, nor perform other operations which are not
    /// directly requested by user. They will still receive incoming frames and emit corresponding
    /// events.
    ///
    /// [`Node::deactivate`] is idempotent.
    pub fn deactivate(&mut self) {
        if self.state.is_closed() {
            return;
        }

        if !self.is_active.is() {
            return;
        }

        self.is_active.set(false);
    }
}

impl<D: Dialect, V: MaybeVersioned, A: NodeApi<V>> Node<Edge<V>, D, V, A> {
    /// MAVLink system ID.
    pub fn system_id(&self) -> SystemId {
        self.kind.endpoint.system_id()
    }

    /// MAVLink component ID.
    pub fn component_id(&self) -> ComponentId {
        self.kind.endpoint.component_id()
    }
}

impl<K: NodeKind, D: Dialect, V: Versioned, A: NodeApi<V>> Node<K, D, V, A> {
    /// MAVLink version.
    pub fn version(&self) -> MavLinkVersion {
        V::version()
    }
}

impl<D: Dialect, A: NodeApi<Versionless>> Node<Edge<Versionless>, D, Versionless, A> {
    /// Send MAVLink frame with a specified MAVLink protocol version.
    ///
    /// If you want to restrict MAVLink protocol to a particular version, construct a [`Versioned`]
    /// node and simply send messages by calling [`Node::send`].
    pub fn send_versioned<V: Versioned>(&self, message: &impl Message) -> Result<()> {
        let frame = self.kind.endpoint.next_frame::<V>(message)?.versionless();
        self.api.send_frame(&frame)
    }
}

impl<K: NodeKind, D: Dialect, V: MaybeVersioned + 'static, A: NodeApi<V>> Drop
    for Node<K, D, V, A>
{
    fn drop(&mut self) {
        self.close()
    }
}
