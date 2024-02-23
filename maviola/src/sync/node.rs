use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::atomic::AtomicU8;
use std::sync::{atomic, Arc, RwLock};
use std::time::Duration;

use crate::core::io::ConnectionInfo;
use crate::core::marker::{
    HasComponentId, HasSystemId, Identified, MaybeIdentified, NoComponentId, NoConnConf,
    NoSystemId, Unidentified,
};
use crate::core::utils::{Guarded, SharedCloser, Switch};
use crate::core::{NodeBuilder, NodeConf};
use crate::protocol::{
    ComponentId, Dialect, Frame, MavLinkVersion, MaybeVersioned, Message, SystemId, Versioned,
    Versionless,
};
use crate::protocol::{Peer, PeerId};
use crate::sync::conn::Connection;
use crate::sync::event::{Event, EventsIterator};
use crate::sync::handler::{HeartbeatEmitter, InactivePeersHandler, IncomingFramesHandler};
use crate::sync::marker::ConnConf;
use crate::sync::Callback;

use crate::prelude::*;

/// MAVLink node.
///
/// # Examples
///
/// Create a TCP server node:
///
/// ```rust
/// # #[cfg(feature = "sync")]
/// # {
/// use maviola::protocol::{Peer, V2};
/// use maviola::sync::{Event, Node, TcpServer};
/// use maviola::dialects::Minimal;
/// # use portpicker::pick_unused_port;
///
/// let addr = "127.0.0.1:5600";
/// # let addr = format!("127.0.0.1:{}", pick_unused_port().unwrap());
///
/// // Create a node from configuration.
/// let mut node = Node::try_from(
///     Node::builder()
///         .version(V2)                // restrict node to MAVLink2 protocol version
///         .system_id(1)               // System `ID`
///         .component_id(1)            // Component `ID`
///         .dialect::<Minimal>()       // Dialect is set to `minimal`
///         .connection(
///             TcpServer::new(addr)    // Configure TCP server connection
///                 .unwrap()
///         )
/// ).unwrap();
///
/// // Activate node to start sending heartbeats.
/// node.activate().unwrap();
/// # struct __Struct(); impl __Struct { fn events(&self) -> Vec<Event<V2>> { vec![Event::NewPeer(Peer::new(0, 0))] } }
/// # let node = __Struct();
///
/// for event in node.events() {
///     match event {
///         Event::NewPeer(peer) => {
///             /* handle a new peer */
/// #           drop(peer);
///         }
///         Event::PeerLost(peer) => {
///             /* handle a peer, that becomes inactive */
/// #           drop(peer);
///         }
///         Event::Frame(frame, res) => {
///             // Send back any incoming frame directly to sender.
///             res.respond(&frame).unwrap();
///         }
///     }
/// }
/// # }
/// ```
pub struct Node<I: MaybeIdentified, D: Dialect, V: MaybeVersioned + 'static> {
    id: I,
    version: V,
    sequence: Arc<AtomicU8>,
    state: SharedCloser,
    is_active: Guarded<SharedCloser, Switch>,
    connection: Connection<V>,
    peers: Arc<RwLock<HashMap<PeerId, Peer>>>,
    heartbeat_timeout: Duration,
    heartbeat_interval: Duration,
    events_tx: mpmc::Sender<Event<V>>,
    events_rx: mpmc::Receiver<Event<V>>,
    _dialect: PhantomData<D>,
}

impl Node<Unidentified, Minimal, Versionless> {
    /// Instantiates an empty [`NodeBuilder`].
    pub fn builder() -> NodeBuilder<NoSystemId, NoComponentId, Minimal, Versionless, NoConnConf> {
        NodeBuilder::new()
    }
}

impl<I: MaybeIdentified, D: Dialect, V: MaybeVersioned + 'static> Node<I, D, V> {
    /// Instantiates node from node configuration.
    ///
    /// Creates ona instance of [`Node`] from [`NodeConf`]. It is also possible to use [`TryFrom`]
    /// and create a node with [`Node::try_from`].
    pub fn try_from_conf(conf: NodeConf<I, D, V, ConnConf<V>>) -> Result<Self> {
        let connection = conf.connection().build()?;
        let state = connection.share_state();
        let is_active = Guarded::from(&state);
        let (events_tx, events_rx) = mpmc::channel();

        let node = Self {
            id: conf.id,
            version: conf.version,
            state,
            is_active,
            sequence: Arc::new(AtomicU8::new(0)),
            connection,
            peers: Default::default(),
            heartbeat_timeout: conf.heartbeat_timeout,
            heartbeat_interval: conf.heartbeat_interval,
            events_tx,
            events_rx,
            _dialect: PhantomData,
        };

        node.start_default_handlers();

        Ok(node)
    }

    /// Information about this node's connection.
    pub fn info(&self) -> &ConnectionInfo {
        self.connection.info()
    }

    /// Heartbeat timeout.
    ///
    /// For peers that overdue to send the next heartbeat within this interval will be considered
    /// inactive. An [`Event::PeerLost`] will be dispatched via [`events`](Node::events),
    /// [`recv_event`](Node::recv_event), and [`try_recv_event`](Node::try_recv_event).
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

    /// Receive MAVLink message blocking until MAVLink frame received.
    pub fn recv(&self) -> Result<(D, Callback<V>)> {
        let (frame, res) = self.recv_frame_internal()?;
        let msg = D::decode(frame.payload())?;
        Ok((msg, res))
    }

    /// Attempts to receive MAVLink message without blocking.
    pub fn try_recv(&self) -> Result<(D, Callback<V>)> {
        let (frame, res) = self.try_recv_frame_internal()?;
        let msg = D::decode(frame.payload())?;
        Ok((msg, res))
    }

    /// Returns an iterator over current peers.
    ///
    /// This method will return a snapshot of the current peers relevant to the time when it was
    /// called. A more reliable approach to peer management is to use [`Node::events`] and track
    /// [`Event::NewPeer`] / [`Event::PeerLost`] events.
    pub fn peers(&self) -> impl Iterator<Item = Peer> {
        let peers: Vec<Peer> = match self.peers.read() {
            Ok(peers) => peers.values().cloned().collect(),
            Err(_) => Vec::new(),
        };

        peers.into_iter()
    }

    /// Returns `true` if node has connected MAVLink peers.
    ///
    /// Disconnected node will always return `false`.
    pub fn has_peers(&self) -> bool {
        match self.peers.read() {
            Ok(peers) => !peers.is_empty(),
            Err(_) => false,
        }
    }

    /// Proxy MAVLink [`Frame`].
    ///
    /// In proxy mode [`Frame`] is sent with as many fields preserved as possible. However, the
    /// following properties could be updated based on the node's
    /// [message signing](https://mavlink.io/en/guide/message_signing.html) configuration
    /// (`MAVLink 2` [`Versioned`] nodes only):
    ///
    /// * [`signature`](Frame::signature)
    /// * [`link_id`](Frame::link_id)
    /// * [`timestamp`](Frame::timestamp)
    ///
    /// To send MAVLink messages instead of raw frames, construct an [`Identified`] node and use
    /// messages [`Node::send_versioned`] for node which is [`Versionless`] and [`Node::send`] for
    /// [`Versioned`] nodes. In the latter case, message will be encoded according to MAVLink
    /// protocol version defined for a node.
    pub fn proxy_frame(&self, frame: &Frame<V>) -> Result<()> {
        self.send_frame_internal(frame)
    }

    /// Receive MAVLink [`Frame`].
    ///
    /// Blocks until frame received.
    pub fn recv_frame(&self) -> Result<(Frame<V>, Callback<V>)> {
        self.recv_frame_internal()
    }

    /// Attempts to receive MAVLink [`Frame`] without blocking.
    pub fn try_recv_frame(&self) -> Result<(Frame<V>, Callback<V>)> {
        self.try_recv_frame_internal()
    }

    /// Request the next node [`Event`].
    ///
    /// Blocks until event received.
    pub fn recv_event(&self) -> Result<Event<V>> {
        self.events_rx.recv().map_err(Error::from)
    }

    /// Attempts to receive MAVLink [`Event`] without blocking.
    pub fn try_recv_event(&self) -> Result<Event<V>> {
        self.events_rx.try_recv().map_err(Error::from)
    }

    /// Subscribe to node events.
    ///
    /// Blocks while the node is active.
    pub fn events(&self) -> impl Iterator<Item = Event<V>> {
        EventsIterator {
            rx: self.events_rx.clone(),
        }
    }

    fn start_default_handlers(&self) {
        self.handle_incoming_frames();
        self.handle_inactive_peers();
    }

    fn handle_incoming_frames(&self) {
        let handler = IncomingFramesHandler {
            info: self.info().clone(),
            peers: self.peers.clone(),
            receiver: self.connection.receiver(),
            events_tx: self.events_tx.clone(),
        };
        handler.spawn(self.state.to_closable());
    }

    fn handle_inactive_peers(&self) {
        let handler = InactivePeersHandler {
            info: self.info().clone(),
            peers: self.peers.clone(),
            timeout: self.heartbeat_timeout,
            events_tx: self.events_tx.clone(),
        };

        handler.spawn(self.state.to_closable());
    }

    fn recv_frame_internal(&self) -> Result<(Frame<V>, Callback<V>)> {
        self.connection.recv()
    }

    fn try_recv_frame_internal(&self) -> Result<(Frame<V>, Callback<V>)> {
        self.connection.try_recv()
    }

    fn send_frame_internal(&self, frame: &Frame<V>) -> Result<()> {
        self.connection.send(frame)
    }
}

impl<D: Dialect, V: Versioned + 'static> Node<Identified, D, V> {
    /// Send MAVLink message.
    ///
    /// The message will be encoded according to the node's dialect specification and MAVLink
    /// protocol version.
    ///
    /// If you want to send messages within different MAVLink protocols simultaneously, you have
    /// to construct a [`Versionless`] node and use [`Node::send_versioned`]
    pub fn send(&self, message: &impl Message) -> Result<()> {
        let frame = self.make_frame_from_message(message, self.version.clone())?;
        self.send_frame_internal(&frame)
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

    /// Activates the node.
    ///
    /// Active nodes emit heartbeats and perform other operations which do not depend on user
    /// initiative directly.
    ///
    /// This method is available only for nodes which are [`Identified`].
    ///
    /// [`Node::activate`] is idempotent while node is connected. Otherwise, it will return
    /// [`NodeError::Inactive`] variant of [`Error::Node`].
    pub fn activate(&mut self) -> Result<()> {
        if self.state.is_closed() {
            return Err(Error::Node(NodeError::Inactive));
        }

        if self.is_active.is() {
            return Ok(());
        }

        self.is_active.set(true);
        self.start_sending_heartbeats();

        Ok(())
    }

    /// Deactivates the node.
    ///
    /// Inactive nodes will neither send heartbeats, nor perform other operations which are not
    /// directly requested by user. They will still receive incoming frames and emit corresponding
    /// events.
    ///
    /// [`Node::deactivate`] is idempotent while node is connected. Otherwise, it will return
    /// [`NodeError::Inactive`] variant of [`Error::Node`].
    pub fn deactivate(&mut self) -> Result<()> {
        if self.state.is_closed() {
            return Err(Error::Node(NodeError::Inactive));
        }

        if !self.is_active.is() {
            return Ok(());
        }

        self.is_active.set(false);

        Ok(())
    }

    fn start_sending_heartbeats(&self) {
        let emitter = HeartbeatEmitter {
            info: self.info().clone(),
            id: self.id.clone(),
            interval: self.heartbeat_interval,
            version: self.version.clone(),
            sender: self.connection.sender(),
            sequence: self.sequence.clone(),
            _dialect: PhantomData::<D>,
        };
        emitter.spawn(self.is_active.clone());
    }
}

impl<D: Dialect, V: MaybeVersioned> Node<Identified, D, V> {
    /// MAVLink system ID.
    pub fn system_id(&self) -> SystemId {
        self.id.system_id
    }

    /// MAVLink component ID.
    pub fn component_id(&self) -> ComponentId {
        self.id.component_id
    }

    fn make_frame_from_message<Version: Versioned>(
        &self,
        message: &impl Message,
        version: Version,
    ) -> Result<Frame<Version>> {
        let sequence = self.sequence.fetch_add(1, atomic::Ordering::Relaxed);
        let payload = message.encode(Version::version())?;
        let frame = Frame::builder()
            .sequence(sequence)
            .system_id(self.id.system_id)
            .component_id(self.id.component_id)
            .payload(payload)
            .crc_extra(message.crc_extra())
            .version(version)
            .build();
        Ok(frame)
    }
}

impl<I: MaybeIdentified, D: Dialect, V: Versioned> Node<I, D, V> {
    /// MAVLink version.
    pub fn version(&self) -> MavLinkVersion {
        V::version()
    }
}

impl<D: Dialect> Node<Identified, D, Versionless> {
    /// Send MAVLink frame with a specified MAVLink protocol version.
    ///
    /// If you want to restrict MAVLink protocol to a particular version, construct a [`Versioned`]
    /// node and simply send messages by calling [`Node::send`].
    pub fn send_versioned<V: Versioned>(&self, message: &impl Message, version: V) -> Result<()> {
        let frame = self
            .make_frame_from_message(message, version)?
            .versionless();
        self.send_frame_internal(&frame)
    }
}

impl<I: MaybeIdentified, D: Dialect, V: MaybeVersioned + 'static>
    TryFrom<NodeConf<I, D, V, ConnConf<V>>> for Node<I, D, V>
{
    type Error = Error;

    /// Attempts to construct [`Node`] from configuration.
    fn try_from(value: NodeConf<I, D, V, ConnConf<V>>) -> Result<Self> {
        Self::try_from_conf(value)
    }
}

impl<D: Dialect, V: MaybeVersioned>
    TryFrom<NodeBuilder<HasSystemId, HasComponentId, D, V, ConnConf<V>>>
    for Node<Identified, D, V>
{
    type Error = Error;

    /// Attempts to construct an identified [`Node`] from a node builder.
    fn try_from(
        value: NodeBuilder<HasSystemId, HasComponentId, D, V, ConnConf<V>>,
    ) -> Result<Self> {
        Self::try_from_conf(value.conf())
    }
}

impl<D: Dialect, V: MaybeVersioned>
    TryFrom<NodeBuilder<NoSystemId, NoComponentId, D, V, ConnConf<V>>>
    for Node<Unidentified, D, V>
{
    type Error = Error;

    /// Attempts to construct an unidentified [`Node`] from a node builder.
    fn try_from(value: NodeBuilder<NoSystemId, NoComponentId, D, V, ConnConf<V>>) -> Result<Self> {
        Self::try_from_conf(value.conf())
    }
}

impl<I: MaybeIdentified, D: Dialect, V: MaybeVersioned + 'static> Drop for Node<I, D, V> {
    fn drop(&mut self) {
        self.state.close();

        log::debug!("{:?}: node is dropped", self.info());
    }
}
