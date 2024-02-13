//! MAVLink node.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU8};
use std::sync::{atomic, mpsc, Arc, Mutex, TryLockError};
use std::thread;

use mavio::protocol::{
    ComponentId, DialectImpl, DialectMessage, Frame, MavLinkVersion, MaybeVersioned, SystemId,
    Versioned, Versionless,
};

use crate::io::node_conf::NodeConf;
use crate::io::node_variants::{Identified, IsIdentified};
use crate::io::sync::connection::{ConnectionConfInfo, ConnectionEvent};
use crate::io::sync::{Connection, ConnectionConf};

use crate::prelude::*;
use crate::protocol::marker::{HasDialect, MaybeDialect};

/// MAVLink node.
pub struct Node<I: IsIdentified, D: MaybeDialect, V: MaybeVersioned + 'static> {
    sequence: AtomicU8,
    id: I,
    dialect: D,
    version: V,
    connections: AtomicConnections<V>,
    recv_rx: mpsc::Receiver<Result<Frame<V>>>,
    info: NodeInfo,
}

impl<I: IsIdentified, D: MaybeDialect, V: MaybeVersioned + 'static> TryFrom<NodeConf<I, D, V>>
    for Node<I, D, V>
{
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

impl<M: DialectMessage + 'static, I: IsIdentified, V: MaybeVersioned> Node<I, HasDialect<M>, V> {
    /// Dialect specification.
    pub fn dialect(&self) -> &'static dyn DialectImpl<Message = M> {
        self.dialect.0
    }
}

impl<I: IsIdentified, D: MaybeDialect, V: Versioned> Node<I, D, V> {
    /// MAVLink version.
    pub fn version(&self) -> MavLinkVersion {
        V::version()
    }
}

impl<M: DialectMessage + 'static, V: Versioned + 'static> Node<Identified, HasDialect<M>, V> {
    /// Send MAVLink message.
    ///
    /// The message will be encoded according to the node's dialect specification and MAVLink
    /// protocol version.
    ///
    /// If you want to send messages within different MAVLink protocols simultaneously, you have
    /// to construct a [`Versionless`] node and use [`Node::send_versioned`]
    pub fn send(&self, message: M) -> Result<usize> {
        let frame = self.make_frame_from_message(message, self.version.clone())?;
        self.send_frame_internal(&frame)
    }
}

impl<M: DialectMessage + 'static> Node<Identified, HasDialect<M>, Versionless> {
    /// Send MAVLink frame with a specified MAVLink protocol version.
    ///
    /// If you want to restrict MAVLink protocol to a particular version, construct a [`Versioned`]
    /// node and simply send messages by calling [`Node::send`].
    pub fn send_versioned<V: Versioned>(&self, message: M, version: V) -> Result<usize> {
        let frame = self
            .make_frame_from_message(message, version)?
            .versionless();
        self.send_frame_internal(&frame)
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

impl<M: DialectMessage + 'static, I: IsIdentified, V: MaybeVersioned + 'static>
    Node<I, HasDialect<M>, V>
{
    /// Receive MAVLink message.
    pub fn recv(&self) -> Result<M> {
        // let frame = self.recv_frame_internal();
        // self.dialect.0.decode(f)
        todo!()
    }
}

impl<I: IsIdentified, D: MaybeDialect, V: MaybeVersioned + 'static> Node<I, D, V> {
    /// Information about this node.
    pub fn info(&self) -> &NodeInfo {
        &self.info
    }

    /// Proxy MAVLink frame.
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
    pub fn proxy_frame(&self, frame: &Frame<V>) -> Result<usize> {
        self.send_frame_internal(frame)
    }

    /// Receive MAVLink frame.
    pub fn recv_frame(&self) -> Result<Frame<V>> {
        self.recv_frame_internal()
    }

    /// Close all connections and stop.
    pub fn close(&self) -> Result<()> {
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

impl<I: IsIdentified, D: MaybeDialect, V: MaybeVersioned + 'static> Node<I, D, V> {
    fn recv_frame_internal(&self) -> Result<Frame<V>> {
        self.recv_rx.recv().map_err(Error::from)?
    }

    fn send_frame_internal(&self, frame: &Frame<V>) -> Result<usize> {
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
        conn_conf: &dyn ConnectionConf<V>,
    ) -> Result<(
        AtomicConnections<V>,
        mpsc::Receiver<Result<Frame<V>>>,
        NodeInfo,
    )> {
        let connections: AtomicConnections<V> = Default::default();
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
        recv_tx: mpsc::Sender<Result<Frame<V>>>,
        connections: AtomicConnections<V>,
        events: mpsc::Receiver<ConnectionEvent<V>>,
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
        connection: Box<dyn Connection<V>>,
        recv_tx: mpsc::Sender<Result<Frame<V>>>,
        connections: &AtomicConnections<V>,
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
        connections: &AtomicConnections<V>,
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
        recv_tx: mpsc::Sender<Result<Frame<V>>>,
        connections: AtomicConnections<V>,
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

impl<I: IsIdentified, D: MaybeDialect, V: MaybeVersioned + 'static> Drop for Node<I, D, V> {
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
struct ManagedConnection<V: MaybeVersioned> {
    connection: Box<dyn Connection<V>>,
    close_tx: mpsc::Sender<()>,
    is_active: AtomicBool,
}

/// Atomic connections which can be shared between threads.
type AtomicConnections<V> = Arc<Mutex<HashMap<usize, ManagedConnection<V>>>>;

impl<V: MaybeVersioned> ManagedConnection<V> {
    fn new(connection: Box<dyn Connection<V>>, close_tx: mpsc::Sender<()>) -> Self {
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

impl<V: MaybeVersioned> Drop for ManagedConnection<V> {
    fn drop(&mut self) {
        if let Err(err) = self.close() {
            log::trace!(
                "{:?}: error closing connection: {err:?}",
                self.connection.info()
            )
        }
    }
}
