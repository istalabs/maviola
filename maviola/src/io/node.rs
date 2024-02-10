//! MAVLink node.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU8};
use std::sync::{atomic, mpsc, Arc, Mutex, TryLockError};
use std::thread;

use mavio::protocol::{ComponentId, DialectImpl, DialectMessage, MavLinkVersion, SystemId};

use crate::prelude::*;

use crate::io::node_conf::NodeConf;
use crate::io::node_variants::{Identified, IsIdentified};
use crate::io::sync::connection::{ConnectionConfInfo, ConnectionEvent};
use crate::io::sync::{Connection, ConnectionConf};
use crate::protocol::variants::{HasDialect, IsDialect, IsVersioned, NotVersioned, Versioned};
use crate::protocol::{CoreFrame, Frame};

/// Interface for MAVLink communication node which can send and receive [`CoreFrame`].
pub trait NodeInterface {
    /// Information about this node.
    fn info(&self) -> &NodeInfo;

    /// Proxy MAVLink frame.
    ///
    /// In proxy mode [`CoreFrame`] is sent with as many fields preserved as possible.
    ///
    /// In particular, the following fields are always preserved:
    ///
    /// * [`sequence`](CoreFrame::sequence)
    /// * [`system_id`](CoreFrame::system_id)
    /// * [`component_id`](CoreFrame::component_id)
    ///
    /// The following properties could be updated based on the node's
    /// [message signing](https://mavlink.io/en/guide/message_signing.html) configuration:
    ///
    /// * [`signature`](CoreFrame::signature)
    /// * [`link_id`](CoreFrame::link_id)
    /// * [`timestamp`](CoreFrame::timestamp)
    ///
    /// # Stateful Sending
    ///
    /// To send frames with correct auto-incremented [`sequence`](Frame::sequence) and
    /// [`system_id`](Node::system_id) / [`component_id`](Node::component_id) bound to a node, you
    /// have to construct an [`Identified`] node and use [`Node::send_frame`].
    ///
    /// To send messages, construct an [`Identified`] node with [`HasDialect`] and
    /// [`Node::send_versioned`]. You can also use [`Node::send`] for [`Versioned`] nodes. In the
    /// latter case, message will be encoded according to MAVLink protocol version defined by for a
    /// node.
    fn proxy_frame(&self, frame: &CoreFrame) -> Result<usize>;

    /// Receive MAVLink frame.
    fn recv_frame(&self) -> Result<CoreFrame>;

    /// Close all connections and stop.
    fn close(&self) -> Result<()>;
}

/// MAVLink node.
pub struct Node<I: IsIdentified, D: IsDialect, V: IsVersioned> {
    sequence: AtomicU8,
    id: I,
    dialect: D,
    version: V,
    connections: AtomicConnections,
    recv_rx: mpsc::Receiver<Result<CoreFrame>>,
    info: NodeInfo,
}

impl<I: IsIdentified, D: IsDialect, V: IsVersioned> TryFrom<NodeConf<I, D, V>> for Node<I, D, V> {
    type Error = Error;

    /// Instantiates [`Node`] from node configuration.
    fn try_from(value: NodeConf<I, D, V>) -> Result<Self> {
        let (connections, recv_rx, info) = Self::start_handlers(value.conn_conf())?;

        Ok(Self {
            sequence: Default::default(),
            id: value.id,
            dialect: value.dialect,
            version: value.version,
            connections,
            recv_rx,
            info,
        })
    }
}

impl<D: IsDialect, V: IsVersioned> Node<Identified, D, V> {
    /// MAVLink system ID.
    pub fn system_id(&self) -> SystemId {
        self.id.system_id
    }

    /// MAVLink component ID.
    pub fn component_id(&self) -> ComponentId {
        self.id.component_id
    }
}

impl<M: DialectMessage + 'static, I: IsIdentified, V: IsVersioned> Node<I, HasDialect<M>, V> {
    /// Dialect specification.
    pub fn dialect(&self) -> &'static dyn DialectImpl<Message = M> {
        self.dialect.0
    }
}

impl<I: IsIdentified, D: IsDialect, V: Versioned> Node<I, D, V> {
    /// MAVLink version.
    pub fn version(&self) -> MavLinkVersion {
        self.version.mavlink_version()
    }
}

impl<M: DialectMessage + 'static, V: Versioned> Node<Identified, HasDialect<M>, V> {
    /// Send MAVLink frame.
    pub fn send(&self, message: M) -> Result<usize> {
        let frame = self.make_frame_from_message(message, self.version.mavlink_version())?;
        self.send_frame_internal(&frame)
    }
}

impl<M: DialectMessage + 'static> Node<Identified, HasDialect<M>, NotVersioned> {
    /// Send MAVLink frame with specified version.
    pub fn send_versioned(&self, message: M, version: MavLinkVersion) -> Result<usize> {
        let frame = self.make_frame_from_message(message, version)?;
        self.send_frame_internal(&frame)
    }
}

impl<D: IsDialect, V: IsVersioned> Node<Identified, D, V> {
    /// Send MAVLink frame.
    ///
    /// Sends [`CoreFrame`] potentially changing its fields.
    ///
    /// Updated fields:
    ///
    /// * [`sequence`](CoreFrame::sequence) - set to the next value
    /// * [`system_id`](CoreFrame::system_id) - set to node's default
    /// * [`component_id`](CoreFrame::component_id) - set to node's default
    ///
    /// The following properties could be updated based on the node's configuration:
    ///
    /// * [`signature`](CoreFrame::signature)
    /// * [`link_id`](CoreFrame::link_id)
    /// * [`timestamp`](CoreFrame::timestamp)
    #[inline]
    pub fn send_frame(&self, frame: &CoreFrame) -> Result<usize> {
        self.send_frame_internal(frame)
    }

    fn make_frame_from_message<M: DialectMessage + 'static>(
        &self,
        message: M,
        version: MavLinkVersion,
    ) -> Result<CoreFrame> {
        let sequence = self.sequence.fetch_add(1, atomic::Ordering::Relaxed);
        let payload = message.encode(version)?;
        CoreFrame::builder()
            .set_sequence(sequence)
            .set_system_id(self.id.system_id)
            .set_component_id(self.id.component_id)
            .set_payload(payload)
            .set_crc_extra(message.crc_extra())
            .build(version)
            .map_err(Error::from)
    }
}

impl<I: IsIdentified, D: IsDialect, V: IsVersioned> Node<I, D, V> {
    /// Receive MAVLink frame.
    pub fn recv(&self) -> Result<Frame<D, V>> {
        let mavio_frame = self.recv_frame()?;
        let frame = Frame::builder()
            .version_generic(self.version.clone())
            .dialect_generic(self.dialect.clone())
            .build_for(mavio_frame)?;
        Ok(frame)
    }
}

impl<I: IsIdentified, D: IsDialect, V: IsVersioned> NodeInterface for Node<I, D, V> {
    fn info(&self) -> &NodeInfo {
        &self.info
    }

    fn proxy_frame(&self, frame: &CoreFrame) -> Result<usize> {
        self.send_frame_internal(frame)
    }

    fn recv_frame(&self) -> Result<CoreFrame> {
        self.recv_frame_internal()
    }

    fn close(&self) -> Result<()> {
        let connections = self.connections.lock()?;
        let mut result = Ok(());

        for connection in connections.values() {
            if let Err(err) = connection.close() {
                if result.is_ok() {
                    result = Err(err);
                }
            }
        }

        result
    }
}

impl<I: IsIdentified, D: IsDialect, V: IsVersioned> Node<I, D, V> {
    fn recv_frame_internal(&self) -> Result<CoreFrame> {
        self.recv_rx.recv().map_err(Error::from)?
    }

    fn send_frame_internal(&self, frame: &CoreFrame) -> Result<usize> {
        let connections = self.connections.lock()?;
        let frame = Arc::new(frame.clone());

        let mut handlers = Vec::new();
        for connection in connections.values() {
            let sender = connection.connection.sender();
            let frame = frame.clone();

            handlers.push(thread::spawn(move || -> Result<usize> {
                sender.lock()?.send(frame.as_ref())
            }));
        }

        let mut bytes_sent = 0;
        for handler in handlers {
            match handler
                .join()
                .map_err(|err| NodeError::Thread(format!("{err:?}")))?
            {
                Ok(num) => {
                    if num > bytes_sent {
                        bytes_sent = num;
                    }
                }
                Err(err) => log::error!("{:?}: can't send frame: {err:?}", self.info.conn),
            }
        }

        Ok(bytes_sent)
    }

    fn start_handlers(
        conn_conf: &dyn ConnectionConf,
    ) -> Result<(
        AtomicConnections,
        mpsc::Receiver<Result<CoreFrame>>,
        NodeInfo,
    )> {
        let connections: AtomicConnections = Default::default();
        let connections_managed = connections.clone();
        let events = conn_conf.build()?;
        let (recv_tx, recv_rx) = mpsc::channel();

        let n_conn = Arc::new(Mutex::new(usize::default()));
        let node_info = NodeInfo {
            conn: conn_conf.info(),
            n_conn: n_conn.clone(),
        };
        let node_info_internal = node_info.clone();

        thread::spawn(move || {
            let conn_info = node_info_internal.conn.clone();
            match Self::handle_conn_events(node_info_internal, recv_tx, connections_managed, events)
            {
                Ok(_) => {}
                Err(err) => {
                    log::error!("{conn_info:?} events handler error: {err:?}")
                }
            }
        });

        Ok((connections, recv_rx, node_info))
    }

    fn handle_conn_events(
        node_info: NodeInfo,
        recv_tx: mpsc::Sender<Result<CoreFrame>>,
        connections: AtomicConnections,
        events: mpsc::Receiver<ConnectionEvent>,
    ) -> Result<()> {
        for event in events {
            match event {
                ConnectionEvent::New(connection) => {
                    Self::handle_connection_event_new(
                        connection,
                        recv_tx.clone(),
                        &connections,
                        &node_info,
                    )?;
                }
                ConnectionEvent::Drop(id, err) => {
                    Self::handle_connection_event_drop(id, &err, &connections, &node_info)?
                }
                _ => continue,
            };
        }
        Ok(())
    }

    fn handle_connection_event_new(
        connection: Box<dyn Connection>,
        recv_tx: mpsc::Sender<Result<CoreFrame>>,
        connections: &AtomicConnections,
        node_info: &NodeInfo,
    ) -> Result<()> {
        let conn_info = &node_info.conn;
        let connections_handled = connections.clone();
        let (close_tx, close_rx) = mpsc::channel();

        let (id, connection_info, new_n_conn) = {
            let mut connections = connections.lock()?;

            let id = connection.id();
            if connections.contains_key(&id) {
                log::error!("{conn_info:?} connection #{id} already exists");
                return Ok(());
            }

            let connection_info = connection.info().clone();
            let managed_connection = ManagedConnection::new(connection, close_tx);

            connections.insert(id, managed_connection);

            (id, connection_info, connections.len())
        };

        {
            let mut n_conn = node_info.n_conn.lock()?;
            *n_conn = new_n_conn;
        }

        log::debug!("{conn_info:?}: add connection #{id}: {:?}", connection_info);

        let conn_info = node_info.clone();
        thread::spawn(move || {
            Self::handle_connection(id, conn_info, recv_tx, connections_handled, close_rx);
        });

        Ok(())
    }

    fn handle_connection_event_drop(
        id: usize,
        err: &Option<Error>,
        connections: &AtomicConnections,
        node_info: &NodeInfo,
    ) -> Result<()> {
        let conn_info = &node_info.conn;

        let (connection, new_n_conn) = {
            let mut connections = connections.lock()?;
            let connection = connections.remove(&id);
            let new_n_conn = connections.len();
            (connection, new_n_conn)
        };

        if let Some(connection) = connection {
            {
                let mut n_conn = node_info.n_conn.lock()?;
                *n_conn = new_n_conn;
            }

            if let Err(err) = connection.close() {
                log::error!("{conn_info:?}: can't close connection #{id}: {err:?}")
            }
            if let Some(err) = err {
                log::debug!(
                    "{conn_info:?}: close connection #{id} {:?} due to error: {err:?}",
                    connection.connection.info()
                );
            };
        }

        Ok(())
    }

    fn handle_connection(
        id: usize,
        node_info: NodeInfo,
        recv_tx: mpsc::Sender<Result<CoreFrame>>,
        connections: AtomicConnections,
        close_rx: mpsc::Receiver<()>,
    ) {
        let conn_info = &node_info.conn;
        let (receiver, id) = match connections.lock() {
            Ok(connections) => match connections.get(&id) {
                None => {
                    log::debug!("{conn_info:?}: connection #{id} is no longer present");
                    return;
                }
                Some(connection) => (connection.connection.receiver(), connection.connection.id()),
            },
            Err(err) => {
                log::error!("{conn_info:?} unable to acquire connections, exiting handler due to error: {err:?}");
                return;
            }
        };

        loop {
            if close_rx.try_recv().is_ok() {
                log::debug!("{conn_info:?}: connection #{id} is closed, exiting handler");
                return;
            }

            match receiver.try_lock() {
                Ok(mut receiver) => {
                    let result = receiver.recv();
                    if let Err(err) = recv_tx.send(result) {
                        log::error!("{conn_info:?}: error passing received frame: {err:?}");
                        return;
                    }
                }
                Err(err) => match err {
                    TryLockError::Poisoned(err) => {
                        log::error!("{conn_info:?} unable to acquire receiver, exiting handler due to error: {err:?}");
                        return;
                    }
                    TryLockError::WouldBlock => {}
                },
            }
        }
    }
}

impl<I: IsIdentified, D: IsDialect, V: IsVersioned> Drop for Node<I, D, V> {
    fn drop(&mut self) {
        if let Err(err) = self.close() {
            log::error!("{:?}: can't close node: {err:?}", &self.info.conn);
        }
    }
}

/// Information about node.
#[derive(Clone, Debug)]
pub struct NodeInfo {
    conn: ConnectionConfInfo,
    n_conn: Arc<Mutex<usize>>,
}

impl NodeInfo {
    /// Connection config.
    pub fn conn(&self) -> &ConnectionConfInfo {
        &self.conn
    }

    /// Approximate number of node connections.
    ///
    /// Blocks to acquire mutex. You should never rely on exact value returned by this method.
    pub fn n_conn(&self) -> Result<usize> {
        Ok(*self
            .n_conn
            .lock()
            .map_err(|err| Error::Node(NodeError::Thread(format!("{err:?}"))))?)
    }
}

/// Connection that can be closed.
struct ManagedConnection {
    connection: Box<dyn Connection>,
    close_tx: mpsc::Sender<()>,
    is_active: AtomicBool,
}

/// Atomic connections which can be shared between threads.
type AtomicConnections = Arc<Mutex<HashMap<usize, ManagedConnection>>>;

impl ManagedConnection {
    fn new(connection: Box<dyn Connection>, close_tx: mpsc::Sender<()>) -> Self {
        Self {
            connection,
            close_tx,
            is_active: AtomicBool::new(true),
        }
    }

    fn close(&self) -> Result<()> {
        if !self.is_active.load(atomic::Ordering::Relaxed) {
            return Ok(());
        }

        self.is_active.store(false, atomic::Ordering::Relaxed);
        self.close_tx.send(())?;
        self.connection.close()
    }
}

impl Drop for ManagedConnection {
    fn drop(&mut self) {
        if let Err(err) = self.close() {
            log::trace!(
                "{:?}: error closing connection: {err:?}",
                self.connection.info()
            )
        }
    }
}
