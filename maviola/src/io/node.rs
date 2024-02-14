//! MAVLink node.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU8};
use std::sync::mpsc::TryRecvError;
use std::sync::{atomic, Arc, RwLock};
use std::thread;
use std::time::{Duration, SystemTime};

use crate::consts::HEARTBEAT_TIMEOUT_TOLERANCE;
use crate::io::event::EventsIterator;
use crate::io::Event;
use mavio::protocol::{
    ComponentId, DialectImpl, DialectMessage, Frame, MavLinkVersion, MaybeVersioned, SystemId,
    Versioned, Versionless,
};

use crate::io::node_conf::NodeConf;
use crate::io::sync::{Connection, ConnectionInfo, Response};
use crate::marker::{HasDialect, Identified, IsIdentified, MaybeDialect};

use crate::prelude::*;
use crate::protocol::{Peer, PeerId};

/// MAVLink node.
pub struct Node<I: IsIdentified, D: MaybeDialect, V: MaybeVersioned + 'static> {
    id: I,
    dialect: D,
    version: V,
    sequence: Arc<AtomicU8>,
    is_active: Arc<AtomicBool>,
    is_started: Arc<AtomicBool>,
    connection: Connection<V>,
    peers: Arc<RwLock<HashMap<PeerId, Peer>>>,
    timeout: Duration,
    events_tx: mpmc::Sender<Event<V>>,
    events_rx: mpmc::Receiver<Event<V>>,
}

impl<I: IsIdentified, D: MaybeDialect, V: MaybeVersioned + 'static> TryFrom<NodeConf<I, D, V>>
    for Node<I, D, V>
{
    type Error = Error;

    /// Instantiates [`Node`] from node configuration.
    fn try_from(value: NodeConf<I, D, V>) -> Result<Self> {
        let connection = value.conn_conf().build()?;
        let (events_tx, events_rx) = mpmc::channel();

        let node = Self {
            id: value.id,
            dialect: value.dialect,
            version: value.version,
            is_active: Arc::new(AtomicBool::new(true)),
            is_started: Arc::new(AtomicBool::new(false)),
            sequence: Arc::new(AtomicU8::new(0)),
            connection,
            peers: Default::default(),
            timeout: value.timeout,
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
        self.is_active.store(false, atomic::Ordering::Relaxed);
        self.is_started.store(false, atomic::Ordering::Relaxed);
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
        let is_active = self.is_active.clone();

        thread::spawn(move || loop {
            if !is_active.load(atomic::Ordering::Relaxed) {
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
        let timeout = self.timeout.mul_f64(HEARTBEAT_TIMEOUT_TOLERANCE);
        let events_tx = self.events_tx.clone();
        let is_active = self.is_active.clone();

        thread::spawn(move || loop {
            if !is_active.load(atomic::Ordering::Relaxed) {
                log::trace!(
                    "[{info:?}] closing inactive peers handler since node is no longer active"
                );
                return;
            }

            thread::sleep(timeout);
            let now = SystemTime::now();

            let inactive_peers = match peers.read() {
                Ok(peers) => {
                    let mut inactive_peers = HashSet::new();
                    for peer in peers.values() {
                        if now.duration_since(peer.last_active).unwrap() > timeout {
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
    /// Starts node handlers.
    ///
    /// This method is available only for nodes which are at the same time [`Identified`],
    /// [`Versioned`], and [`HasDialect`].
    pub fn start(&self) -> Result<()> {
        if !self.is_active.load(atomic::Ordering::Relaxed) {
            return Err(Error::Node(NodeError::Inactive));
        }

        if self.is_started.load(atomic::Ordering::Relaxed) {
            return Ok(());
        }

        self.is_started.store(true, atomic::Ordering::Relaxed);
        self.start_heartbeat_handler();

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

    fn start_heartbeat_handler(&self) {
        use mavio::dialects::minimal as dialect;

        let is_active = self.is_active.clone();
        let info = self.info().clone();
        let sequence = self.sequence.clone();
        let system_id = self.system_id();
        let component_id = self.component_id();
        let timeout = self.timeout;
        let version = self.version.clone();
        let heartbeat = dialect::messages::Heartbeat {
            type_: Default::default(),
            autopilot: dialect::enums::MavAutopilot::Generic,
            base_mode: Default::default(),
            custom_mode: 0,
            system_status: dialect::enums::MavState::Active,
            mavlink_version: self.dialect.0.version().unwrap_or_default(),
        };
        let connection = self.connection.clone();

        thread::spawn(move || loop {
            if !is_active.load(atomic::Ordering::Relaxed) {
                log::trace!("[{info:?}] closing heartbeat sender since node is no longer active");
                return;
            }

            let sequence = sequence.fetch_add(1, atomic::Ordering::Relaxed);
            let frame = Frame::builder()
                .sequence(sequence)
                .system_id(system_id)
                .component_id(component_id)
                .version(version.clone())
                .message(&heartbeat)
                .unwrap()
                .build();

            log::trace!("[{info:?}] broadcasting heartbeat");
            if let Err(err) = connection.send(&frame) {
                log::error!("[{info:?}] heartbeat can't be sent: {err:?}");
                return;
            }

            thread::sleep(timeout);
        });
    }
}

impl<I: IsIdentified, D: MaybeDialect, V: MaybeVersioned + 'static> Drop for Node<I, D, V> {
    fn drop(&mut self) {
        if let Err(err) = self.close() {
            log::error!("{:?}: can't close node: {err:?}", self.info());
        }
    }
}
