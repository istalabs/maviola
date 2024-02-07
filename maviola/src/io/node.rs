//! MAVLink node.

use std::collections::HashMap;
use std::sync::atomic::AtomicU8;
use std::sync::{atomic, mpsc, Arc, Mutex};
use std::thread;

use mavio::protocol::{DialectImpl, DialectMessage, MavLinkVersion};
use mavio::Frame;

use crate::errors::{Error, Result};
use crate::io::node_conf::builder::{
    NodeConfBuilder, WithComponentId, WithConnectionConf, WithSystemId,
};
use crate::io::node_conf::variants::{WithDialect, WithId};
use crate::io::node_conf::NodeConf;
use crate::io::sync::connection::ConnectionEvent;
use crate::io::sync::Connection;

/// MAVLink node.
pub struct Node<M: DialectMessage + 'static> {
    sequence: AtomicU8,
    system_id: u8,
    component_id: u8,
    dialect: &'static dyn DialectImpl<Message = M>,
    connections: AtomicConnections,
    recv_rx: mpsc::Receiver<Result<Frame>>,
}

impl<M: DialectMessage + 'static> TryFrom<NodeConf<WithId, WithDialect, M>> for Node<M> {
    type Error = Error;

    /// Instantiates [`Node`] from node configuration with specified identity, dialect, and
    /// connection config.
    fn try_from(value: NodeConf<WithId, WithDialect, M>) -> Result<Self> {
        let connections: AtomicConnections = Default::default();
        let connections_managed = connections.clone();
        let events = value.conn_conf().build()?;
        let (recv_tx, recv_rx) = mpsc::channel();

        thread::spawn(move || {
            Self::handle_conn_events(recv_tx, connections_managed, events);
        });

        Ok(Self {
            sequence: Default::default(),
            system_id: value.system_id(),
            component_id: value.component_id(),
            dialect: value.dialect(),
            connections,
            recv_rx,
        })
    }
}

impl<M: DialectMessage + 'static>
    TryFrom<NodeConfBuilder<WithSystemId, WithComponentId, WithConnectionConf, WithDialect, M>>
    for Node<M>
{
    type Error = Error;

    /// Instantiates [`Node`] from node configuration builder with specified identity, dialect, and
    /// connection config.
    fn try_from(
        value: NodeConfBuilder<WithSystemId, WithComponentId, WithConnectionConf, WithDialect, M>,
    ) -> Result<Self> {
        Self::try_from(value.build())
    }
}

impl<M: DialectMessage + 'static> Node<M> {
    /// Sends message.
    pub fn send(&self, message: M) -> Result<usize> {
        let sequence = self.sequence.fetch_add(1, atomic::Ordering::Relaxed);
        let payload = message.encode(MavLinkVersion::V2)?;
        let frame = Frame::builder()
            .set_message_id(payload.id())
            .set_sequence(sequence)
            .set_system_id(self.system_id)
            .set_component_id(self.component_id)
            .set_payload(payload)
            .set_crc_extra(message.crc_extra())
            .build(MavLinkVersion::V2)?;

        let connections = loop {
            if let Ok(connections) = self.connections.lock() {
                break connections;
            }
        };

        let mut bytes_sent = 0;
        for connection in connections.values() {
            bytes_sent = connection.conn.sender().lock().unwrap().send(&frame)?;
        }

        Ok(bytes_sent)
    }

    /// Receives message.
    pub fn recv(&self) -> Result<M> {
        let frame = self.recv_rx.recv().map_err(Error::from)??;
        self.dialect.decode(frame.payload()).map_err(Error::from)
    }

    /// Close all connections and stop.
    pub fn close(&self) -> Result<()> {
        loop {
            if let Ok(connections) = self.connections.lock() {
                let mut result = Ok(());
                for connection in connections.values() {
                    if let Err(err) = connection.close() {
                        result = Err(err);
                    }
                }
                return result;
            }
        }
    }

    fn handle_conn_events(
        recv_tx: mpsc::Sender<Result<Frame>>,
        connections: AtomicConnections,
        events: mpsc::Receiver<ConnectionEvent>,
    ) {
        for event in events {
            match event {
                ConnectionEvent::New(connection) => loop {
                    let recv_tx = recv_tx.clone();
                    let connections_handled = connections.clone();

                    let (close_tx, close_rx) = mpsc::channel();

                    if let Ok(mut connections) = connections.lock() {
                        let id = connection.id();
                        connections.insert(
                            id,
                            ManagedConnection {
                                conn: connection,
                                close_tx,
                            },
                        );

                        thread::spawn(move || {
                            Self::handle_connection(id, recv_tx, connections_handled, close_rx);
                        });

                        break;
                    }
                },
                ConnectionEvent::Drop(id, err) => loop {
                    if let Ok(mut connections) = connections.lock() {
                        if let Some(connection) = connections.remove(&id) {
                            log::warn!(
                                "remove connection: {:?} due to {err:?}",
                                connection.conn.info()
                            );
                        } else {
                            log::error!("can't remove connection #{id}")
                        }
                        break;
                    }
                },
                _ => continue,
            };
        }
    }

    fn handle_connection(
        id: usize,
        recv_tx: mpsc::Sender<Result<Frame>>,
        connections: AtomicConnections,
        close_rx: mpsc::Receiver<()>,
    ) {
        let (receiver, id) = loop {
            if let Ok(connections) = connections.lock() {
                match connections.get(&id) {
                    None => {
                        log::debug!("connection #{id} is no longer present");
                        return;
                    }
                    Some(connection) => break (connection.conn.receiver(), connection.conn.id()),
                }
            }
        };

        loop {
            if close_rx.try_recv().is_ok() {
                log::debug!("connection #{id} is closed");
                return;
            }

            if let Ok(mut receiver) = receiver.lock() {
                if let Err(err) = recv_tx.send(receiver.recv()) {
                    log::error!("error passing received frame: {err:?}");
                    return;
                }
            };
        }
    }
}

/// Connection that can be closed.
struct ManagedConnection {
    conn: Box<dyn Connection>,
    close_tx: mpsc::Sender<()>,
}

/// Atomic connections.
type AtomicConnections = Arc<Mutex<HashMap<usize, ManagedConnection>>>;

impl ManagedConnection {
    /// Closes managed connection and sends close signal to all handlers.
    fn close(&self) -> Result<()> {
        self.close_tx.send(())?;
        self.conn.close()
    }
}

#[cfg(test)]
mod sync_node_tests {
    use std::collections::HashMap;
    use std::thread;
    use std::time::Duration;

    use crate::dialects::minimal;
    use crate::io::node::Node;
    use crate::io::node_conf::NodeConf;
    use crate::io::sync::{TcpClientConf, TcpServerConf};

    fn make_server_node(port: portpicker::Port) -> Node<minimal::Message> {
        Node::try_from(
            NodeConf::builder()
                .set_system_id(1)
                .set_component_id(1)
                .set_dialect(minimal::dialect())
                .set_conn_conf(TcpServerConf::new(format!("127.0.0.1:{port}")).unwrap()),
        )
        .unwrap()
    }

    fn make_client_node(port: portpicker::Port, component_id: u8) -> Node<minimal::Message> {
        Node::try_from(
            NodeConf::builder()
                .set_system_id(2)
                .set_component_id(component_id)
                .set_dialect(minimal::dialect())
                .set_conn_conf(TcpClientConf::new(format!("127.0.0.1:{port}")).unwrap()),
        )
        .unwrap()
    }

    fn make_client_nodes(port: portpicker::Port, count: u8) -> HashMap<u8, Node<minimal::Message>> {
        (0..count).map(|i| (i, make_client_node(port, i))).collect()
    }

    fn unused_port() -> portpicker::Port {
        portpicker::pick_unused_port().unwrap()
    }

    #[test]
    fn new_connections_are_handled() {
        let port = unused_port();
        let server_node = make_server_node(port);

        const CLIENT_COUNT: usize = 5;
        let client_nodes = make_client_nodes(port, CLIENT_COUNT as u8);

        // Wait long enough
        thread::sleep(Duration::from_millis(5));

        {
            let connections = server_node.connections.lock().unwrap();
            assert_eq!(connections.len(), CLIENT_COUNT);
        }

        for client_node in client_nodes.values() {
            let connections = client_node.connections.lock().unwrap();
            assert_eq!(connections.len(), 1);
        }
    }

    #[test]
    fn messages_are_sent_and_received() {
        let port = unused_port();
        let server_node = make_server_node(port);

        const CLIENT_COUNT: usize = 5;
        let client_nodes = make_client_nodes(port, CLIENT_COUNT as u8);

        // Wait long enough
        thread::sleep(Duration::from_millis(5));

        let message = minimal::Message::Heartbeat(minimal::messages::Heartbeat::default());
        server_node.send(message).unwrap();

        for client_node in client_nodes.values() {
            if let minimal::Message::Heartbeat(recv_message) = client_node.recv().unwrap() {
                println!("{recv_message:#?}");
            } else {
                panic!("invalid message")
            }
        }
    }

    #[test]
    fn closed_connections_are_dropped() {
        let port = unused_port();
        let server_node = make_server_node(port);
        let client_node = make_client_node(port, 0);

        // Wait long enough
        thread::sleep(Duration::from_millis(5));

        client_node.close().unwrap();

        // Wait long enough
        thread::sleep(Duration::from_millis(5));

        let message = minimal::Message::Heartbeat(minimal::messages::Heartbeat::default());
        server_node.send(message).unwrap();

        // Wait long enough
        thread::sleep(Duration::from_millis(50));

        {
            let connections = server_node.connections.lock().unwrap();
            assert_eq!(connections.len(), 0);
        }
    }
}
