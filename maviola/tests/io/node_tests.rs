use std::collections::HashMap;
use std::sync::Once;
use std::thread;
use std::time::Duration;

use mavio::protocol::{MavLinkVersion, V2};

use maviola::dialects::minimal;
use maviola::io::node::Node;
use maviola::io::node_conf::NodeConf;
use maviola::io::node_variants::Identified;
use maviola::io::sync::{TcpClientConf, TcpServerConf};
use maviola::protocol::variants::HasDialect;

static INIT: Once = Once::new();
const LOG_LEVEL: log::LevelFilter = log::LevelFilter::Info;
const WAIT_DURATION: Duration = Duration::from_millis(5);
const HOST: &str = "127.0.0.1";

fn unused_port() -> portpicker::Port {
    portpicker::pick_unused_port().unwrap()
}

fn make_addr(port: portpicker::Port) -> String {
    format!("{HOST}:{port}")
}

fn wait() {
    thread::sleep(WAIT_DURATION)
}

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

fn make_server_node(port: portpicker::Port) -> Node<Identified, HasDialect<minimal::Message>, V2> {
    Node::try_from(
        NodeConf::builder()
            .system_id(1)
            .component_id(1)
            .dialect(minimal::dialect())
            .v2()
            .conn_conf(TcpServerConf::new(make_addr(port)).unwrap())
            .build(),
    )
    .unwrap()
}

fn make_client_node(
    port: portpicker::Port,
    component_id: u8,
) -> Node<Identified, HasDialect<minimal::Message>, V2> {
    Node::try_from(
        NodeConf::builder()
            .system_id(2)
            .component_id(component_id)
            .dialect(minimal::dialect())
            .v2()
            .conn_conf(TcpClientConf::new(make_addr(port)).unwrap())
            .build(),
    )
    .unwrap()
}

fn make_client_nodes(
    port: portpicker::Port,
    count: u8,
) -> HashMap<u8, Node<Identified, HasDialect<minimal::Message>, V2>> {
    (0..count).map(|i| (i, make_client_node(port, i))).collect()
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
        let n_con = server_node.info().n_conn().unwrap();
        assert_eq!(n_con, CLIENT_COUNT);
    }

    for client_node in client_nodes.values() {
        let n_con = client_node.info().n_conn().unwrap();
        assert_eq!(n_con, 1);
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
        let n_con = server_node.info().n_conn().unwrap();
        assert_eq!(n_con, 0);
    }
}

#[test]
fn node_no_id_no_dialect_no_version() {
    initialize();

    let port = unused_port();
    let server_node = make_server_node(port);

    let client_node = Node::try_from(
        NodeConf::builder()
            .conn_conf(TcpClientConf::new(make_addr(port)).unwrap())
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
        .sequence(sequence)
        .system_id(system_id)
        .component_id(component_id)
        .mavlink_version(V2)
        .message(&minimal::messages::Heartbeat::default())
        .unwrap()
        .build();

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
            .dialect(minimal::dialect())
            .conn_conf(TcpClientConf::new(make_addr(port)).unwrap())
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
            .dialect(minimal::dialect())
            .v2()
            .conn_conf(TcpClientConf::new(make_addr(port)).unwrap())
            .build(),
    )
    .unwrap();

    wait();

    server_node
        .send(minimal::messages::Heartbeat::default().into())
        .unwrap();
    client_node
        .recv()
        .unwrap()
        .decode::<minimal::Message>()
        .unwrap();
}

#[test]
fn node_no_version() {
    initialize();

    let port = unused_port();
    let server_node = make_server_node(port);

    let client_node = Node::try_from(
        NodeConf::builder()
            .system_id(42)
            .component_id(142)
            .v2()
            .dialect(minimal::dialect())
            .conn_conf(TcpClientConf::new(make_addr(port)).unwrap())
            .build(),
    )
    .unwrap();

    wait();

    client_node
        .send_versioned(minimal::messages::Heartbeat::default().into(), V2)
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
