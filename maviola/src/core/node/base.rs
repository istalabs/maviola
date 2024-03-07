use std::marker::PhantomData;
use std::time::Duration;

use crate::core::io::ConnectionInfo;
use crate::core::marker::{Edge, NoComponentId, NoConnConf, NoSystemId, NodeKind, Proxy};
use crate::core::node::api::{NoApi, NodeApi};
use crate::core::node::NodeBuilder;
use crate::core::utils::{Guarded, SharedCloser, Switch};
use crate::protocol::{
    ComponentId, Dialect, MavLinkVersion, MaybeVersioned, Message, MessageSigner, SystemId,
    Versioned, Versionless,
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
/// to know `CRC_EXTRA` of the exact message this frame encodes. You can validate incoming frames
/// against node dialect using [`Node::validate_frame`], or use [`Frame::validate_checksum`] to
/// validate against external dialect.
///
/// ## Message Signing
///
/// MAVLink [message signing](https://mavlink.io/en/guide/message_signing.html) is provided by
/// [`MessageSigner`] that can be provided upon node configuration. It can be configured with
/// incoming and outgoing [`SignStrategy`] for a fine-grained control over what and when should be
/// signed. It is possible to validate several authenticated links with additional keys, but only
/// one link `ID` / key pair will be used to sign frames.
///
/// ## Examples
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
///     .dialect::<Minimal>()       // Dialect is set to `minimal`
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
///     .dialect::<Minimal>()       // Dialect is set to `minimal`
///     .id(MavLinkId::new(1, 1))   // Set system and component IDs
///     .async_connection(
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
/// let node = Node::builder()
///     .version(V2)
///     .id(MavLinkId::new(1, 1))
///     .connection(TcpServer::new("127.0.0.1:5600").unwrap())
///     .signer(
///         MessageSigner::builder()
///             .link_id(1)
///             .key("unsecure")
///             .incoming(SignStrategy::Strict)
///             .outgoing(SignStrategy::Sign)
///             .build()
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

    /// Returns a reference to [`MessageSigner`], that is responsible for
    /// [message signing](https://mavlink.io/en/guide/message_signing.html).
    #[inline(always)]
    pub fn signer(&self) -> Option<&MessageSigner> {
        self.api.signer()
    }

    /// Returns `true` if node is connected.
    ///
    /// All nodes are connected by default, they can become disconnected only if I/O transport
    /// failed or have been exhausted.
    pub fn is_connected(&self) -> bool {
        !self.state.is_closed()
    }

    /// Send MAVLink [`Frame`].
    ///
    /// The [`Frame`] will be sent with as many fields preserved as possible. However, the
    /// following properties could be updated based on the [`Node::signer`] configuration
    /// (`MAVLink 2` frames only):
    ///
    /// * [`signature`](Frame::signature)
    /// * [`link_id`](Frame::link_id)
    /// * [`timestamp`](Frame::timestamp)
    ///
    /// To send MAVLink messages instead of raw frames, construct an [`Edge`] node and use
    /// [`Node::send_versioned`] for node which is [`Versionless`] and [`Node::send`] for
    /// [`Versioned`] nodes. In the latter case, message will be encoded according to MAVLink
    /// protocol version defined for a node.
    pub fn send_frame(&self, frame: &Frame<V>) -> Result<()> {
        self.api.send_frame(frame)
    }

    /// Attempts to send versionless MAVLink frame.
    ///
    /// Similar to [`Node::send_frame`], except it accepts only [`Versionless`] frames and tries
    /// to convert them into a supported versioned form. If conversion is not possible, the method
    /// will return [`FrameError::InvalidVersion`] variant of [`Error::Frame`].
    pub fn send_versionless_frame(&self, frame: &Frame<Versionless>) -> Result<()> {
        let frame = frame.try_to_versioned::<V>()?;
        self.send_frame(&frame)
    }

    /// Validates incoming frame using node configuration.
    pub fn validate_frame(&self, frame: &Frame<V>) -> Result<()> {
        frame.validate_checksum::<D>()?;
        Ok(())
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
        let frame = self.next_frame(message)?;
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

    /// Creates a next frame from MAVLink message.
    ///
    /// If [`Node::signer`] is set and the node has `MAVLink 2` protocol version, then frame will
    /// be signed according to the [`MessageSigner::outgoing`] strategy.
    pub fn next_frame(&self, message: &impl Message) -> Result<Frame<V>> {
        let mut frame = self.kind.endpoint.next_frame(message)?;

        if let Some(signer) = self.signer() {
            signer.process_new(&mut frame);
        }

        Ok(frame)
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
        let frame = self.next_frame_versioned::<V>(message)?;
        self.api.send_frame(&frame)
    }

    /// Create a next frame from MAVLink message with a specified protocol version.
    ///
    /// After creation, the frame will be converted into a [`Versionless`] form.
    pub fn next_frame_versioned<V: Versioned>(
        &self,
        message: &impl Message,
    ) -> Result<Frame<Versionless>> {
        let mut frame = self.kind.endpoint.next_frame::<V>(message)?;

        if let Some(signer) = self.signer() {
            signer.process_new(&mut frame);
        }

        Ok(frame)
    }
}

impl<K: NodeKind, D: Dialect, V: MaybeVersioned + 'static, A: NodeApi<V>> Drop
    for Node<K, D, V, A>
{
    fn drop(&mut self) {
        self.close()
    }
}
