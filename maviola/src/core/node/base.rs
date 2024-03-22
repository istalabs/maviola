use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;

use crate::core::io::{BroadcastScope, ConnectionInfo};
use crate::core::marker::{Edge, NodeKind, Proxy, Unset};
use crate::core::node::{NodeApi, NodeBuilder, SendFrameInternal, SendMessageInternal};
use crate::core::utils::{Guarded, Sealed, SharedCloser, Switch};
use crate::protocol::{ComponentId, DialectSpec, FrameProcessor, SystemId};

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
/// ## Sending and Receiving
///
/// The suggested approach for receiving incoming frames is to use [`Node::events`]. This method
/// returns an iterator (or a stream in the case of asynchronous API) over node events, such as
/// incoming frames, invalid frames, that hasn't passed signature validation, new peers, and so on.
///
/// You can also receive individual frames via [`Node::recv_frame`] / [`Node::try_recv_frame`].
///
/// To send messages you may use either [`Node::send`], or [`Node::send_frame`]. The former accepts
/// MAVLink messages and decodes them into frames, the latter one is sending MAVLink frames
/// directly.
///
/// ## Frame Validation
///
/// Since MAVLink is a connectionless protocol, the only way to ensure frame consistency, is to
/// validate its checksum. The checksum serves two purposes: first, it ensures, that message was
/// not damaged during sending, and second, it guarantees that sender and receiver use the same
/// version of a dialect. Unfortunately, that means, that in order to validate a frame, you have
/// to know `CRC_EXTRA` of the exact message this frame encodes. Frames will be validated according
/// to [`Node::known_dialects`] upon sending and receiving. Invalid incoming frames will be
/// available as corresponding events. You can always validate frames against arbitrary dialect
/// using [`Frame::validate_checksum`].
///
/// ## Message Signing
///
/// MAVLink [message signing](https://mavlink.io/en/guide/message_signing.html) is provided by
/// [`FrameSigner`] that can be provided upon node configuration. It can be configured with
/// incoming and outgoing [`SignStrategy`] for a fine-grained control over what and when should be
/// signed. It is possible to validate several authenticated links with additional keys, but only
/// one link `ID` / key pair will be used to sign frames.
///
/// ## Multiple Connections
///
/// It is possible to create a node with multiple connections. There is special transport called
/// [`Network`], that encapsulates several proxy nodes. Such nodes may have individual setting,
/// such as message signing configuration.
///
/// ## Examples
///
/// Create a synchronous TCP server node:
///
/// ```rust,no_run
/// use maviola::prelude::*;
/// use maviola::sync::prelude::*;
///
/// let addr = "127.0.0.1:5600";
///
/// // Create a node from synchronous configuration
/// // with MAVLink protocol set to `V2`
/// let mut node = Node::sync::<V2>()
///     .id(MavLinkId::new(1, 1))   // Set system and component IDs
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
/// ```rust,no_run
/// # #[tokio::main(flavor = "current_thread")] async fn main() {
/// use maviola::prelude::*;
/// use maviola::asnc::prelude::*;
///
/// let addr = "127.0.0.1:5600";
///
/// // Create a node from asynchronous configuration
/// // with MAVLink protocol set to `V2`
/// let mut node = Node::asnc::<V2>()
///     .id(MavLinkId::new(1, 1))   // Set system and component IDs
///     .connection(
///         TcpServer::new(addr)    // Configure TCP server connection
///             .unwrap()
///     ).build().await.unwrap();
///
/// // Activate node to start sending heartbeats
/// node.activate().await.unwrap();
/// # }
/// ```
///
/// Create a synchronous node, that signs all outgoing messages and rejects unsigned or incorrectly
/// signed incoming messages:
///
/// ```rust,no_run
/// use maviola::dialects::minimal::messages::Heartbeat;
/// use maviola::prelude::*;
/// use maviola::sync::prelude::*;
///
/// let node = Node::sync::<V2>()
///     .id(MavLinkId::new(1, 1))
///     .connection(TcpServer::new("127.0.0.1:5600").unwrap())
///     .signer(
///         FrameSigner::builder()
///             // Set `ID` of a signed link
///             .link_id(1)
///             // Set secret key
///             .key("unsecure")
///             // Reject unsigned or incorrect incoming messages
///             .incoming(SignStrategy::Strict)
///             // Sign all outgoing messages
///             .outgoing(SignStrategy::Sign)
///     )
///     .build().unwrap();
///
/// // The following message will be signed during sending
/// node.send(&Heartbeat::default()).unwrap();
///
/// // Incoming frames are always correctly signed
/// let (frame, _) = node.recv_frame().unwrap();
/// assert!(frame.is_signed());
/// ```
///
/// Create an asynchronous node with a network containing two TCP servers:
///
/// ```rust,no_run
/// # #[tokio::main] async fn main() {
/// use maviola::prelude::*;
/// use maviola::asnc::prelude::*;
///
/// let node = Node::asnc::<V2>()
///     .id(MavLinkId::new(1, 17))
///     .connection(
///         Network::asnc()
///             .add_connection(TcpServer::new("127.0.0.1:5600").unwrap())
///             .add_connection(TcpServer::new("127.0.0.1:5601").unwrap())
///     )
///     .build().await.unwrap();
/// # }
/// ```
pub struct Node<K: NodeKind, V: MaybeVersioned, A: NodeApi<V>> {
    pub(crate) kind: K,
    pub(crate) api: A,
    pub(crate) state: SharedCloser,
    pub(crate) is_active: Guarded<SharedCloser, Switch>,
    pub(crate) heartbeat_timeout: Duration,
    pub(crate) heartbeat_interval: Duration,
    pub(crate) processor: Arc<FrameProcessor>,
    pub(crate) _version: PhantomData<V>,
}

impl Node<Proxy, Versionless, Unset> {
    /// Instantiates an empty [`NodeBuilder`].
    pub fn builder() -> NodeBuilder<Unset, Unset, Versionless, Unset, Unset> {
        NodeBuilder::new()
    }
}

impl<K: NodeKind, V: MaybeVersioned, A: NodeApi<V>> Node<K, V, A> {
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

    /// Dialect specification.
    ///
    /// Default dialect is [`DefaultDialect`].
    #[inline]
    pub fn dialect(&self) -> &DialectSpec {
        self.processor.main_dialect()
    }

    /// Known dialects specifications.
    ///
    /// Node can perform frame validation against known dialects. However, automatic operations,
    /// like heartbeats, will use the main [`Node::dialect`].
    ///
    /// Main dialect is always among the known dialects.
    pub fn known_dialects(&self) -> impl Iterator<Item = &DialectSpec> {
        self.processor.known_dialects()
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

        log::debug!("[{:?}]: node is closed", self.info());
    }
}

impl<V: Versioned, A: NodeApi<V>> Node<Edge<V>, V, A> {
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

impl<V: MaybeVersioned, A: NodeApi<V>> Node<Edge<V>, V, A> {
    /// MAVLink system ID.
    pub fn system_id(&self) -> SystemId {
        self.kind.endpoint.system_id()
    }

    /// MAVLink component ID.
    pub fn component_id(&self) -> ComponentId {
        self.kind.endpoint.component_id()
    }
}

impl<K: NodeKind, V: Versioned, A: NodeApi<V>> Node<K, V, A> {
    /// MAVLink version.
    pub fn version(&self) -> MavLinkVersion {
        V::version()
    }
}

impl<K: NodeKind, V: MaybeVersioned, A: NodeApi<V>> SendFrameInternal<V> for Node<K, V, A> {
    fn processor(&self) -> &FrameProcessor {
        self.api.processor()
    }

    unsafe fn route_frame_internal(&self, frame: Frame<V>, scope: BroadcastScope) -> Result<()> {
        self.api.route_frame_internal(frame, scope)
    }
}

impl<V: MaybeVersioned, A: NodeApi<V>> SendMessageInternal<V> for Node<Edge<V>, V, A> {
    fn endpoint(&self) -> &Endpoint<V> {
        &self.kind.endpoint
    }
}

impl<K: NodeKind, V: MaybeVersioned, A: NodeApi<V>> Sealed for Node<K, V, A> {}
impl<K: NodeKind, V: MaybeVersioned, A: NodeApi<V>> SendFrame<V> for Node<K, V, A> {}
impl<V: Versioned, A: NodeApi<V>> SendMessage<V> for Node<Edge<V>, V, A> {}
impl<A: NodeApi<Versionless>> SendVersionlessMessage for Node<Edge<Versionless>, Versionless, A> {}

impl<K: NodeKind, V: MaybeVersioned, A: NodeApi<V>> Drop for Node<K, V, A> {
    fn drop(&mut self) {
        self.close()
    }
}
