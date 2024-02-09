//! MAVLink node.

use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, AtomicU8};
use std::sync::{atomic, mpsc, Arc, Mutex, TryLockError};
use std::thread;

use crate::errors::NodeError;
use mavio::protocol::{DialectImpl, DialectMessage, MavLinkVersion};

use crate::prelude::*;

use crate::io::node_conf::NodeConf;
use crate::io::node_variants::{
    Dialect, HasDialect, HasIdentifier, HasVersion, Identified, NoDialect, NotVersioned, Versioned,
};
use crate::io::sync::connection::{ConnectionConfInfo, ConnectionEvent};
use crate::io::sync::{Connection, ConnectionConf};
use crate::protocol::Frame;

/// Interface for MAVLink communication node which can send and receive [`mavio::Frame`].
pub trait NodeInterface {
    /// Node connection info.
    fn connection_info(&self) -> &ConnectionConfInfo;

    /// Proxy MAVLink frame.
    ///
    /// In proxy mode [`mavio::Frame`] is sent with as many fields preserved as possible.
    ///
    /// In particular, the following fields are always preserved:
    ///
    /// * [`sequence`](mavio::Frame::sequence)
    /// * [`system_id`](mavio::Frame::system_id)
    /// * [`component_id`](mavio::Frame::component_id)
    ///
    /// The following properties could be updated based on the node's
    /// [message signing](https://mavlink.io/en/guide/message_signing.html) configuration:
    ///
    /// * [`signature`](mavio::Frame::signature)
    /// * [`link_id`](mavio::Frame::link_id)
    /// * [`timestamp`](mavio::Frame::timestamp)
    ///
    /// # Stateful Sending
    ///
    /// To send frames with correct auto-incremented [`sequence`](mavio::Frame::sequence) and
    /// [`system_id`](Node::system_id) / [`component_id`](Node::component_id) bound to a node, you have
    /// to construct an [`Identified`] node and use [`Node::send_frame`].
    ///
    /// To send messages, construct an [`Identified`] node with [`Dialect`] and [`Node::send_versioned`]. You can also
    /// use [`Node::send`] for [`Versioned`] nodes. In the latter case, message will be encoded according to MAVLink
    /// protocol version defined by for a node.
    fn proxy_frame(&self, frame: &mavio::Frame) -> Result<usize>;

    /// Receive MAVLink frame.
    fn recv_frame(&self) -> Result<mavio::Frame>;

    /// Close all connections and stop.
    fn close(&self) -> Result<()>;
}

/// MAVLink node.
pub struct Node<M: DialectMessage + 'static, I: HasIdentifier, D: HasDialect, V: HasVersion> {
    sequence: AtomicU8,
    id: I,
    dialect: D,
    version: V,
    connections: AtomicConnections,
    recv_rx: mpsc::Receiver<Result<mavio::Frame>>,
    connection_info: ConnectionConfInfo,
    _marker_message: PhantomData<M>,
}

impl<M: DialectMessage + 'static, I: HasIdentifier, D: HasDialect, V: HasVersion> Drop
    for Node<M, I, D, V>
{
    fn drop(&mut self) {
        if let Err(err) = self.close() {
            log::error!("{:?}: can't close node: {err:?}", &self.connection_info);
        }
    }
}

impl<M: DialectMessage + 'static, I: HasIdentifier, D: HasDialect, V: HasVersion>
    TryFrom<NodeConf<I, D, V, M>> for Node<M, I, D, V>
{
    type Error = Error;

    /// Instantiates [`Node`] from node configuration.
    fn try_from(value: NodeConf<I, D, V, M>) -> Result<Self> {
        let (connections, recv_rx, connection_info) = Self::start_handlers(value.conn_conf())?;

        Ok(Self {
            sequence: Default::default(),
            id: value.id,
            dialect: value.dialect,
            version: value.version,
            connections,
            recv_rx,
            connection_info,
            _marker_message: Default::default(),
        })
    }
}

impl<M: DialectMessage + 'static, D: HasDialect, V: HasVersion> Node<M, Identified, D, V> {
    /// MAVLink system ID.
    pub fn system_id(&self) -> u8 {
        self.id.system_id
    }

    /// MAVLink component ID.
    pub fn component_id(&self) -> u8 {
        self.id.component_id
    }
}

impl<M: DialectMessage + 'static> Node<M, Identified, Dialect<M>, Versioned> {
    /// Send MAVLink frame.
    pub fn send(&self, message: M) -> Result<usize> {
        let frame = self.make_frame_from_message(message, self.version.0)?;
        self.send_frame_internal(&frame)
    }
}

impl<M: DialectMessage + 'static> Node<M, Identified, Dialect<M>, NotVersioned> {
    /// Send MAVLink frame with specified version.
    pub fn send_versioned(&self, message: M, version: MavLinkVersion) -> Result<usize> {
        let frame = self.make_frame_from_message(message, version)?;
        self.send_frame_internal(&frame)
    }
}

impl<M: DialectMessage + 'static, D: HasDialect, V: HasVersion> Node<M, Identified, D, V> {
    /// Send MAVLink frame.
    ///
    /// Sends [`mavio::Frame`] potentially changing its fields.
    ///
    /// Updated fields:
    ///
    /// * [`sequence`](mavio::Frame::sequence) - set to the next value
    /// * [`system_id`](mavio::Frame::system_id) - set to node's default
    /// * [`component_id`](mavio::Frame::component_id) - set to node's default
    ///
    /// The following properties could be updated based on the node's configuration:
    ///
    /// * [`signature`](mavio::Frame::signature)
    /// * [`link_id`](mavio::Frame::link_id)
    /// * [`timestamp`](mavio::Frame::timestamp)
    #[inline]
    pub fn send_frame(&self, frame: &mavio::Frame) -> Result<usize> {
        self.send_frame_internal(frame)
    }

    fn make_frame_from_message(&self, message: M, version: MavLinkVersion) -> Result<mavio::Frame> {
        let sequence = self.sequence.fetch_add(1, atomic::Ordering::Relaxed);
        let payload = message.encode(version)?;
        mavio::Frame::builder()
            .set_sequence(sequence)
            .set_system_id(self.id.system_id)
            .set_component_id(self.id.component_id)
            .set_payload(payload)
            .set_crc_extra(message.crc_extra())
            .build(version)
            .map_err(Error::from)
    }
}

impl<M: DialectMessage + 'static, I: HasIdentifier, V: HasVersion> Node<M, I, Dialect<M>, V> {
    /// Dialect specification.
    pub fn dialect(&self) -> &'static dyn DialectImpl<Message = M> {
        self.dialect.0
    }

    /// Receive message.
    pub fn recv(&self) -> Result<Frame<M, Dialect<M>>> {
        let frame = self.recv_frame()?;
        Ok(Frame::with_dialect(frame, self.dialect.0))
    }
}

impl<M: DialectMessage + 'static, I: HasIdentifier, V: HasVersion> Node<M, I, NoDialect, V> {
    /// Receive message.
    pub fn recv(&self) -> Result<Frame<M, NoDialect>> {
        let frame = self.recv_frame()?;
        Ok(Frame::new(frame))
    }
}

impl<M: DialectMessage + 'static, I: HasIdentifier, D: HasDialect> Node<M, I, D, Versioned> {
    /// MAVLink version.
    pub fn version(&self) -> &MavLinkVersion {
        &self.version.0
    }
}

impl<M: DialectMessage + 'static, I: HasIdentifier, D: HasDialect, V: HasVersion> NodeInterface
    for Node<M, I, D, V>
{
    fn connection_info(&self) -> &ConnectionConfInfo {
        &self.connection_info
    }
    fn proxy_frame(&self, frame: &mavio::Frame) -> Result<usize> {
        self.send_frame_internal(frame)
    }

    fn recv_frame(&self) -> Result<mavio::Frame> {
        self.recv_rx.recv().map_err(Error::from)?
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

impl<M: DialectMessage + 'static, I: HasIdentifier, D: HasDialect, V: HasVersion> Node<M, I, D, V> {
    fn send_frame_internal(&self, frame: &mavio::Frame) -> Result<usize> {
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
                Err(err) => log::error!("{:?}: can't send frame: {err:?}", self.connection_info),
            }
        }

        Ok(bytes_sent)
    }

    fn start_handlers(
        conn_conf: &dyn ConnectionConf,
    ) -> Result<(
        AtomicConnections,
        mpsc::Receiver<Result<mavio::Frame>>,
        ConnectionConfInfo,
    )> {
        let connections: AtomicConnections = Default::default();
        let connections_managed = connections.clone();
        let events = conn_conf.build()?;
        let (recv_tx, recv_rx) = mpsc::channel();
        let connection_info = conn_conf.info();
        let node_connection_info = connection_info.clone();

        thread::spawn(move || {
            let conn_info = connection_info.clone();
            match Self::handle_conn_events(connection_info, recv_tx, connections_managed, events) {
                Ok(_) => {}
                Err(err) => {
                    log::error!("{conn_info:?} events handler error: {err:?}")
                }
            }
        });

        Ok((connections, recv_rx, node_connection_info))
    }

    fn handle_conn_events(
        node_conn_info: ConnectionConfInfo,
        recv_tx: mpsc::Sender<Result<mavio::Frame>>,
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
                        &node_conn_info,
                    )?;
                }
                ConnectionEvent::Drop(id, err) => {
                    Self::handle_connection_event_drop(id, &err, &connections, &node_conn_info)?
                }
                _ => continue,
            };
        }
        Ok(())
    }

    fn handle_connection_event_new(
        connection: Box<dyn Connection>,
        recv_tx: mpsc::Sender<Result<mavio::Frame>>,
        connections: &AtomicConnections,
        node_conn_info: &ConnectionConfInfo,
    ) -> Result<()> {
        let connections_handled = connections.clone();
        let (close_tx, close_rx) = mpsc::channel();
        let mut connections = connections.lock()?;

        let id = connection.id();
        if connections.contains_key(&id) {
            log::error!("{node_conn_info:?} connection #{id} already exists");
            return Ok(());
        }

        let connection_info = connection.info().clone();
        let managed_connection = ManagedConnection::new(connection, close_tx);
        connections.insert(id, managed_connection);
        drop(connections);

        log::debug!(
            "{node_conn_info:?}: add connection #{id}: {:?}",
            connection_info
        );

        let conn_info = node_conn_info.clone();
        thread::spawn(move || {
            Self::handle_connection(id, conn_info, recv_tx, connections_handled, close_rx);
        });

        Ok(())
    }

    fn handle_connection_event_drop(
        id: usize,
        err: &Option<Error>,
        connections: &AtomicConnections,
        node_conn_info: &ConnectionConfInfo,
    ) -> Result<()> {
        let mut connections = connections.lock()?;

        if let Some(connection) = connections.remove(&id) {
            if let Err(err) = connection.close() {
                log::error!("{node_conn_info:?}: can't close connection #{id}: {err:?}")
            }
            if let Some(err) = err {
                log::debug!(
                    "{node_conn_info:?}: close connection #{id} {:?} due to error: {err:?}",
                    connection.connection.info()
                );
            };
        }

        Ok(())
    }

    fn handle_connection(
        id: usize,
        conn_info: ConnectionConfInfo,
        recv_tx: mpsc::Sender<Result<mavio::Frame>>,
        connections: AtomicConnections,
        close_rx: mpsc::Receiver<()>,
    ) {
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

/// Connection that can be closed.
struct ManagedConnection {
    connection: Box<dyn Connection>,
    close_tx: mpsc::Sender<()>,
    is_active: AtomicBool,
}

/// Atomic connections.
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

#[cfg(test)]
mod sync_node_tests {
    use mavio::protocol::MavLinkVersion;
    use std::collections::HashMap;
    use std::sync::Once;
    use std::thread;
    use std::time::Duration;

    use crate::dialects::minimal;
    use crate::io::node_conf::NodeConf;
    use crate::io::node_variants::{Dialect, Identified, Versioned};
    use crate::io::sync::{TcpClientConf, TcpServerConf};

    use super::*;

    static INIT: Once = Once::new();
    const LOG_LEVEL: log::LevelFilter = log::LevelFilter::Debug;

    fn initialize() {
        INIT.call_once(|| {
            env_logger::builder()
                // Suppress everything below `warn` for third-party modules
                .filter_level(log::LevelFilter::Warn)
                // Allow everything above `LOG_LEVEL` from current package
                .filter_module(env!("CARGO_PKG_NAME"), LOG_LEVEL)
                .init();
        });
    }

    fn make_server_node(
        port: portpicker::Port,
    ) -> Node<minimal::Message, Identified, Dialect<minimal::Message>, Versioned> {
        Node::try_from(
            NodeConf::builder()
                .set_system_id(1)
                .set_component_id(1)
                .set_dialect(minimal::dialect())
                .set_version(MavLinkVersion::V2)
                .set_conn_conf(TcpServerConf::new(make_addr(port)).unwrap())
                .build(),
        )
        .unwrap()
    }

    fn make_client_node(
        port: portpicker::Port,
        component_id: u8,
    ) -> Node<minimal::Message, Identified, Dialect<minimal::Message>, Versioned> {
        Node::try_from(
            NodeConf::builder()
                .set_system_id(2)
                .set_component_id(component_id)
                .set_dialect(minimal::dialect())
                .set_version(MavLinkVersion::V2)
                .set_conn_conf(TcpClientConf::new(make_addr(port)).unwrap())
                .build(),
        )
        .unwrap()
    }

    fn make_client_nodes(
        port: portpicker::Port,
        count: u8,
    ) -> HashMap<u8, Node<minimal::Message, Identified, Dialect<minimal::Message>, Versioned>> {
        (0..count).map(|i| (i, make_client_node(port, i))).collect()
    }

    fn unused_port() -> portpicker::Port {
        portpicker::pick_unused_port().unwrap()
    }

    fn make_addr(port: portpicker::Port) -> String {
        format!("127.0.0.1:{port}")
    }

    fn wait() {
        thread::sleep(Duration::from_millis(5))
    }

    #[test]
    fn new_connections_are_handled() {
        initialize();

        let port = unused_port();
        let server_node = make_server_node(port);
        const CLIENT_COUNT: usize = 5;
        let client_nodes = make_client_nodes(port, CLIENT_COUNT as u8);
        wait();

        {
            let connections = server_node.connections.try_lock().unwrap();
            assert_eq!(connections.len(), CLIENT_COUNT);
        }

        for client_node in client_nodes.values() {
            let connections = client_node.connections.try_lock().unwrap();
            assert_eq!(connections.len(), 1);
        }
    }

    #[test]
    fn messages_are_sent_and_received() {
        initialize();

        let port = unused_port();
        let server_node = make_server_node(port);
        const CLIENT_COUNT: usize = 5;
        let client_nodes = make_client_nodes(port, CLIENT_COUNT as u8);
        wait();

        let message = minimal::messages::Heartbeat::default();
        server_node.send(message.into()).unwrap();

        for client_node in client_nodes.values() {
            let frame = client_node.recv().unwrap();
            log::info!("{frame:#?}");
            if let minimal::Message::Heartbeat(recv_message) = frame.decode().unwrap() {
                log::info!("{recv_message:#?}");
            } else {
                panic!("invalid message")
            }
        }
    }

    #[test]
    fn closed_connections_are_dropped() {
        initialize();

        let port = unused_port();
        let server_node = make_server_node(port);
        let client_node = make_client_node(port, 0);
        wait();

        client_node.close().unwrap();
        wait();

        let message = minimal::messages::Heartbeat::default();
        server_node.send(message.into()).unwrap();
        wait();

        {
            let connections = server_node.connections.try_lock().unwrap();
            assert_eq!(connections.len(), 0);
        }
    }

    #[test]
    fn node_no_id_no_dialect_no_version() {
        initialize();

        let port = unused_port();
        let server_node = make_server_node(port);

        let client_node = Node::try_from(
            NodeConf::builder()
                .set_conn_conf(TcpClientConf::new(make_addr(port)).unwrap())
                .build(),
        )
        .unwrap();

        wait();

        server_node
            .send(minimal::messages::Heartbeat::default().into())
            .unwrap();
        client_node.recv_frame().unwrap();

        let sequence: u8 = 190;
        let system_id: u8 = 42;
        let component_id: u8 = 142;
        let frame = mavio::Frame::builder()
            .set_sequence(sequence)
            .set_system_id(system_id)
            .set_component_id(component_id)
            .build_for(&minimal::messages::Heartbeat::default(), MavLinkVersion::V2)
            .unwrap();

        for _ in 0..5 {
            client_node.proxy_frame(&frame).unwrap();
        }

        for _ in 0..5 {
            let frame = server_node.recv().unwrap();
            assert_eq!(frame.sequence(), sequence);
            assert_eq!(frame.system_id(), system_id);
            assert_eq!(frame.component_id(), component_id);
        }
    }

    #[test]
    fn node_no_id_no_version() {
        initialize();

        let port = unused_port();
        let server_node = make_server_node(port);

        let client_node = Node::try_from(
            NodeConf::builder()
                .set_dialect(minimal::dialect())
                .set_conn_conf(TcpClientConf::new(make_addr(port)).unwrap())
                .build(),
        )
        .unwrap();

        wait();

        server_node
            .send(minimal::messages::Heartbeat::default().into())
            .unwrap();
        client_node.recv().unwrap();
    }

    #[test]
    fn node_no_id() {
        initialize();

        let port = unused_port();
        let server_node = make_server_node(port);

        let client_node = Node::try_from(
            NodeConf::builder()
                .set_dialect(minimal::dialect())
                .set_version(MavLinkVersion::V2)
                .set_conn_conf(TcpClientConf::new(make_addr(port)).unwrap())
                .build(),
        )
        .unwrap();

        wait();

        server_node
            .send(minimal::messages::Heartbeat::default().into())
            .unwrap();
        client_node.recv().unwrap().decode().unwrap();
    }

    #[test]
    fn node_no_version() {
        initialize();

        let port = unused_port();
        let server_node = make_server_node(port);

        let client_node = Node::try_from(
            NodeConf::builder()
                .set_system_id(42)
                .set_component_id(142)
                .set_dialect(minimal::dialect())
                .set_conn_conf(TcpClientConf::new(make_addr(port)).unwrap())
                .build(),
        )
        .unwrap();

        wait();

        client_node
            .send_versioned(
                minimal::messages::Heartbeat::default().into(),
                MavLinkVersion::V2,
            )
            .unwrap();

        let frame = server_node.recv().unwrap();
        assert_eq!(frame.system_id(), 42);
        assert_eq!(frame.component_id(), 142);
        assert!(matches!(frame.mavlink_version(), MavLinkVersion::V2));

        let message = frame.decode().unwrap();
        if let minimal::Message::Heartbeat(_) = message {
            // message is fine
        } else {
            panic!("Invalid message!")
        }
    }
}
