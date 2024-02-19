use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU8};
use std::sync::mpsc::TryRecvError;
use std::sync::{atomic, Arc, RwLock};
use std::thread;
use std::time::{Duration, SystemTime};

use mavio::protocol::{
    ComponentId, DialectImpl, DialectMessage, Frame, MavLinkVersion, MaybeVersioned, SystemId,
    Versioned, Versionless,
};

use crate::io::node_conf::NodeConf;
use crate::io::sync::event::EventsIterator;
use crate::io::sync::{Connection, Response};
use crate::io::ConnectionInfo;
use crate::io::Event;
use crate::marker::{HasDialect, Identified, IsIdentified, MaybeDialect, SyncConnConf};
use crate::protocol::{Peer, PeerId};

use crate::prelude::*;

/// Synchronous MAVLink node.
///
/// # Examples
///
/// Create a TCP server node:
///
/// ```rust
/// # use maviola::protocol::Peer;
/// # #[cfg(feature = "sync")]
/// # {
/// use maviola::protocol::V2;
/// use maviola::{Event, Node, NodeConf, TcpServerConf};
/// use maviola::dialects::minimal;
/// # use portpicker::pick_unused_port;
///
/// let addr = "127.0.0.1:5600";
/// # let addr = format!("127.0.0.1:{}", pick_unused_port().unwrap());
///
/// // Create a node from configuration.
/// let node = Node::try_from(
///     NodeConf::builder()
///         .version(V2)                    // restrict node to MAVLink2 protocol version
///         .system_id(1)                   // System `ID`
///         .component_id(1)                // Component `ID`
///         .dialect(minimal::dialect())    // Dialect is set to `minimal`
///         .connection(
///             TcpServerConf::new(addr)    // Configure TCP server connection
///                 .unwrap()
///         )
///         .build()
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
pub struct Node<I: IsIdentified, D: MaybeDialect, V: MaybeVersioned + 'static> {
    id: I,
    dialect: D,
    version: V,
    sequence: Arc<AtomicU8>,
    is_connected: Arc<AtomicBool>,
    is_active: Arc<AtomicBool>,
    connection: Connection<V>,
    peers: Arc<RwLock<HashMap<PeerId, Peer>>>,
    heartbeat_timeout: Duration,
    heartbeat_interval: Duration,
    events_tx: mpmc::Sender<Event<V>>,
    events_rx: mpmc::Receiver<Event<V>>,
}

impl<I: IsIdentified, D: MaybeDialect, V: MaybeVersioned + 'static>
    TryFrom<NodeConf<I, D, V, SyncConnConf<V>>> for Node<I, D, V>
{
    type Error = Error;

    /// Instantiates [`Node`] from node configuration.
    fn try_from(value: NodeConf<I, D, V, SyncConnConf<V>>) -> Result<Self> {
        let connection = value.connection().build()?;
        let (events_tx, events_rx) = mpmc::channel();

        let node = Self {
            id: value.id,
            dialect: value.dialect,
            version: value.version,
            is_connected: Arc::new(AtomicBool::new(true)),
            is_active: Arc::new(AtomicBool::new(false)),
            sequence: Arc::new(AtomicU8::new(0)),
            connection,
            peers: Default::default(),
            heartbeat_timeout: value.heartbeat_timeout,
            heartbeat_interval: value.heartbeat_interval,
            events_tx,
            events_rx,
        };

        node.start_default_handlers();

        Ok(node)
    }
}

impl<I: IsIdentified, D: MaybeDialect, V: MaybeVersioned + 'static> Node<I, D, V> {
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
    /// Default value is [`DEFAULT_HEARTBEAT_TIMEOUT`](crate::consts::DEFAULT_HEARTBEAT_TIMEOUT).
    pub fn heartbeat_timeout(&self) -> Duration {
        self.heartbeat_timeout
    }

    /// Returns `true` if node is connected.
    ///
    /// All nodes are connected by default. If node wasn't disconnected by [`Node::close`], then it
    /// can become disconnected only when underlying I/O transport failed or exhausted.
    pub fn is_connected(&self) -> bool {
        self.is_connected.load(atomic::Ordering::Relaxed)
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
    /// To send messages, construct an [`Identified`] node with [`HasDialect`] and send messages via
    /// [`Node::send_versioned`]. You can also use generic [`Node::send`] for [`Versioned`] nodes.
    /// In the latter case, message will be encoded according to MAVLink protocol version defined
    /// by for a node.
    pub fn proxy_frame(&self, frame: &Frame<V>) -> Result<()> {
        self.send_frame_internal(frame)
    }

    /// Receive MAVLink [`Frame`].
    ///
    /// Blocks until frame received.
    pub fn recv_frame(&self) -> Result<(Frame<V>, Response<V>)> {
        self.recv_frame_internal()
    }

    /// Attempts to receive MAVLink [`Frame`] without blocking.
    pub fn try_recv_frame(&self) -> Result<(Frame<V>, Response<V>)> {
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

    /// Close all connections and stop.
    pub fn close(&mut self) -> Result<()> {
        self.is_connected.store(false, atomic::Ordering::Relaxed);
        self.is_active.store(false, atomic::Ordering::Relaxed);
        self.connection.close();

        self.peers
            .write()
            .map_err(Error::from)
            .map(|mut peers| peers.clear())?;

        Ok(())
    }

    fn start_default_handlers(&self) {
        self.handle_incoming_frames();
        self.handle_inactive_peers();
    }

    fn handle_incoming_frames(&self) {
        let info = self.info().clone();
        let connection = self.connection.clone();
        let peers = self.peers.clone();
        let events_tx = self.events_tx.clone();
        let is_connected = self.is_connected.clone();

        thread::spawn(move || loop {
            if !is_connected.load(atomic::Ordering::Relaxed) {
                log::trace!(
                    "[{info:?}] closing incoming frames handler since node is no longer active"
                );
                return;
            }

            let (frame, response) = match connection.try_recv() {
                Ok((frame, resp)) => (frame, resp),
                Err(Error::Sync(SyncError::TryRecv(err))) => match err {
                    TryRecvError::Empty => continue,
                    TryRecvError::Disconnected => {
                        log::trace!("[{info:?}] node connection closed");
                        return;
                    }
                },
                Err(err) => {
                    log::error!("[{info:?}] unhandled node error: {err}");
                    return;
                }
            };

            if let Ok(crate::dialects::Minimal::Heartbeat(_)) = frame.decode() {
                let peer = Peer::new(frame.system_id(), frame.component_id());
                log::trace!("[{info:?}] received heartbeat from {peer:?}");

                match peers.write() {
                    Ok(mut peers) => {
                        let has_peer = peers.contains_key(&peer.id);
                        peers.insert(peer.id, peer.clone());

                        if !has_peer {
                            if let Err(err) = events_tx.send(Event::NewPeer(peer)) {
                                log::error!("[{info:?}] failed to report new peer event: {err}");
                                return;
                            }
                        }
                    }
                    Err(err) => {
                        log::error!("[{info:?}] received {peer:?} but node is offline: {err:?}");
                        return;
                    }
                }
            }

            if let Err(err) = events_tx.send(Event::Frame(frame.clone(), response)) {
                log::error!("[{info:?}] failed to report incoming frame event: {err}");
                return;
            }
        });
    }

    fn handle_inactive_peers(&self) {
        let info = self.info().clone();
        let peers = self.peers.clone();
        let heartbeat_timeout = self.heartbeat_timeout;
        let events_tx = self.events_tx.clone();
        let is_connected = self.is_connected.clone();

        thread::spawn(move || loop {
            if !is_connected.load(atomic::Ordering::Relaxed) {
                log::trace!(
                    "[{info:?}] closing inactive peers handler since node is no longer active"
                );
                return;
            }

            thread::sleep(heartbeat_timeout);
            let now = SystemTime::now();

            let inactive_peers = match peers.read() {
                Ok(peers) => {
                    let mut inactive_peers = HashSet::new();
                    for peer in peers.values() {
                        if now.duration_since(peer.last_active).unwrap() > heartbeat_timeout {
                            inactive_peers.insert(peer.id);
                        }
                    }
                    inactive_peers
                }
                Err(err) => {
                    log::error!("[{info:?}] stopping heartbeat checks: {err:?}");
                    return;
                }
            };

            match peers.write() {
                Ok(mut peers) => {
                    for id in inactive_peers {
                        if let Some(peer) = peers.remove(&id) {
                            if let Err(err) = events_tx.send(Event::PeerLost(peer)) {
                                log::error!("[{info:?}] failed to report lost peer event: {err}");
                                return;
                            }
                        }
                    }
                }
                Err(err) => {
                    log::error!("[{info:?}] stopping heartbeat checks: {err:?}");
                    return;
                }
            }
        });
    }

    fn recv_frame_internal(&self) -> Result<(Frame<V>, Response<V>)> {
        self.connection.recv()
    }

    fn try_recv_frame_internal(&self) -> Result<(Frame<V>, Response<V>)> {
        self.connection.try_recv()
    }

    fn send_frame_internal(&self, frame: &Frame<V>) -> Result<()> {
        self.connection.send(frame)
    }
}

impl<D: MaybeDialect, V: MaybeVersioned> Node<Identified, D, V> {
    /// MAVLink system ID.
    pub fn system_id(&self) -> SystemId {
        self.id.system_id
    }

    /// MAVLink component ID.
    pub fn component_id(&self) -> ComponentId {
        self.id.component_id
    }
}

impl<I: IsIdentified, D: MaybeDialect, V: Versioned> Node<I, D, V> {
    /// MAVLink version.
    pub fn version(&self) -> MavLinkVersion {
        V::version()
    }
}

impl<M: DialectMessage + 'static, I: IsIdentified, V: MaybeVersioned + 'static>
    Node<I, HasDialect<M>, V>
{
    /// Dialect specification.
    pub fn dialect(&self) -> &'static dyn DialectImpl<Message = M> {
        self.dialect.0
    }

    /// Receive MAVLink message blocking until MAVLink frame received.
    pub fn recv(&self) -> Result<(M, Response<V>)> {
        let (frame, res) = self.recv_frame_internal()?;
        let msg = self.dialect.0.decode(frame.payload())?;
        Ok((msg, res))
    }

    /// Attempts to receive MAVLink message without blocking.
    pub fn try_recv(&self) -> Result<(M, Response<V>)> {
        let (frame, res) = self.try_recv_frame_internal()?;
        let msg = self.dialect.0.decode(frame.payload())?;
        Ok((msg, res))
    }
}

impl<M: DialectMessage + 'static, V: MaybeVersioned + 'static> Node<Identified, HasDialect<M>, V> {
    fn make_frame_from_message<Version: Versioned>(
        &self,
        message: M,
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

impl<M: DialectMessage + 'static> Node<Identified, HasDialect<M>, Versionless> {
    /// Send MAVLink frame with a specified MAVLink protocol version.
    ///
    /// If you want to restrict MAVLink protocol to a particular version, construct a [`Versioned`]
    /// node and simply send messages by calling [`Node::send`].
    pub fn send_versioned<V: Versioned>(&self, message: M, version: V) -> Result<()> {
        let frame = self
            .make_frame_from_message(message, version)?
            .versionless();
        self.send_frame_internal(&frame)
    }
}

impl<M: DialectMessage + 'static, V: Versioned + 'static> Node<Identified, HasDialect<M>, V> {
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
        self.is_active.load(atomic::Ordering::Relaxed)
    }

    /// Heartbeat interval.
    ///
    /// Once node is started using [`Node::activate`], it will emit heartbeats with this interval.
    ///
    /// Default value is [`DEFAULT_HEARTBEAT_INTERVAL`](crate::consts::DEFAULT_HEARTBEAT_INTERVAL).
    pub fn heartbeat_interval(&self) -> Duration {
        self.heartbeat_interval
    }

    /// Activates the node.
    ///
    /// Active nodes emit heartbeats and perform other operations which do not depend on user
    /// initiative directly.
    ///
    /// This method is available only for nodes which are at the same time [`Identified`],
    /// [`Versioned`], and [`HasDialect`].
    ///
    /// [`Node::activate`] is idempotent while node is connected. Otherwise, it will return
    /// [`NodeError::Inactive`] variant of [`Error::Node`].
    pub fn activate(&self) -> Result<()> {
        if !self.is_connected.load(atomic::Ordering::Relaxed) {
            return Err(Error::Node(NodeError::Inactive));
        }

        if self.is_active.load(atomic::Ordering::Relaxed) {
            return Ok(());
        }

        self.is_active.store(true, atomic::Ordering::Relaxed);
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
    pub fn deactivate(&self) -> Result<()> {
        if !self.is_connected.load(atomic::Ordering::Relaxed) {
            return Err(Error::Node(NodeError::Inactive));
        }

        if !self.is_active.load(atomic::Ordering::Relaxed) {
            return Ok(());
        }

        self.is_active.store(false, atomic::Ordering::Relaxed);

        Ok(())
    }

    /// Send MAVLink message.
    ///
    /// The message will be encoded according to the node's dialect specification and MAVLink
    /// protocol version.
    ///
    /// If you want to send messages within different MAVLink protocols simultaneously, you have
    /// to construct a [`Versionless`] node and use [`Node::send_versioned`]
    pub fn send(&self, message: M) -> Result<()> {
        let frame = self.make_frame_from_message(message, self.version.clone())?;
        self.send_frame_internal(&frame)
    }

    fn start_sending_heartbeats(&self) {
        let is_connected = self.is_connected.clone();
        let is_active = self.is_active.clone();
        let info = self.info().clone();
        let heartbeat_interval = self.heartbeat_interval;
        let version = self.version.clone();
        let connection = self.connection.clone();

        let sequence = self.sequence.clone();
        let system_id = self.system_id();
        let component_id = self.component_id();

        let heartbeat_message = self.make_heartbeat_message();

        thread::spawn(move || loop {
            if !is_connected.load(atomic::Ordering::Relaxed) {
                log::trace!(
                    "[{info:?}] closing heartbeat emitter since node is no longer connected"
                );
                return;
            }
            if !is_active.load(atomic::Ordering::Relaxed) {
                log::trace!("[{info:?}] closing heartbeat emitter since node is no longer active");
                return;
            }

            let sequence = sequence.fetch_add(1, atomic::Ordering::Relaxed);
            let frame = Frame::builder()
                .sequence(sequence)
                .system_id(system_id)
                .component_id(component_id)
                .version(version.clone())
                .message(&heartbeat_message)
                .unwrap()
                .build();

            log::trace!("[{info:?}] broadcasting heartbeat");
            if let Err(err) = connection.send(&frame) {
                log::error!("[{info:?}] heartbeat can't be broadcast: {err:?}");
                return;
            }

            thread::sleep(heartbeat_interval);
        });
    }

    fn make_heartbeat_message(&self) -> mavio::dialects::minimal::messages::Heartbeat {
        use mavio::dialects::minimal as dialect;

        dialect::messages::Heartbeat {
            type_: Default::default(),
            autopilot: dialect::enums::MavAutopilot::Generic,
            base_mode: Default::default(),
            custom_mode: 0,
            system_status: dialect::enums::MavState::Active,
            mavlink_version: self.dialect.0.version().unwrap_or_default(),
        }
    }
}

impl<I: IsIdentified, D: MaybeDialect, V: MaybeVersioned + 'static> Drop for Node<I, D, V> {
    fn drop(&mut self) {
        if let Err(err) = self.close() {
            log::error!("{:?}: can't close node: {err:?}", self.info());
        }
    }
}
