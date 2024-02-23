use mavio::dialects::Minimal;
use std::collections::HashMap;
use std::sync::Once;
use std::thread;
use std::time::Duration;

use portpicker::Port;

use maviola::core::marker::Identified;
use maviola::dialects::minimal;
use maviola::protocol::{ComponentId, MavLinkVersion, SystemId, V2};
use maviola::sync::{Event, Node};
use maviola::sync::{TcpClient, TcpServer};

static INIT: Once = Once::new();
static INIT_LOGGER: Once = Once::new();
pub const LOG_LEVEL: log::LevelFilter = log::LevelFilter::Debug;
pub const HOST: &str = "127.0.0.1";
const WAIT_DURATION: Duration = Duration::from_millis(50);
const WAIT_LONG_DURATION: Duration = Duration::from_millis(200);
pub const DEFAULT_TCP_SERVER_SYS_ID: SystemId = 2;
pub const DEFAULT_TCP_SERVER_COMP_ID: ComponentId = 0;
pub const DEFAULT_TCP_CLIENT_SYS_ID: SystemId = 20;

fn unused_port() -> Port {
    portpicker::pick_unused_port().unwrap()
}

fn make_addr(port: Port) -> String {
    format!("{HOST}:{port}")
}

fn wait() {
    thread::sleep(WAIT_DURATION)
}

fn wait_long() {
    thread::sleep(WAIT_LONG_DURATION)
}

fn init_logger() {
    INIT_LOGGER.call_once(|| {
        env_logger::builder()
            // Suppress everything below `warn` for third-party modules
            .filter_level(log::LevelFilter::Warn)
            // Allow everything above `LOG_LEVEL` from current package
            .filter_module(env!("CARGO_PKG_NAME"), LOG_LEVEL)
            .init();
    });
}

fn initialize() {
    INIT.call_once(|| init_logger());
}

pub fn make_tcp_server_node(port: Port) -> Node<Identified, Minimal, V2> {
    Node::try_from(
        Node::builder()
            .system_id(DEFAULT_TCP_SERVER_SYS_ID)
            .component_id(DEFAULT_TCP_SERVER_COMP_ID)
            .dialect::<Minimal>()
            .version(V2)
            .connection(TcpServer::new(make_addr(port)).unwrap()),
    )
    .unwrap()
}

pub fn make_tcp_client_node(port: Port, component_id: u8) -> Node<Identified, Minimal, V2> {
    Node::try_from(
        Node::builder()
            .system_id(DEFAULT_TCP_CLIENT_SYS_ID)
            .component_id(component_id)
            .dialect::<Minimal>()
            .version(V2)
            .connection(TcpClient::new(make_addr(port)).unwrap()),
    )
    .unwrap()
}

fn make_client_nodes(
    port: portpicker::Port,
    count: u8,
) -> HashMap<u8, Node<Identified, Minimal, V2>> {
    (0..count)
        .map(|i| (i, make_tcp_client_node(port, i)))
        .collect()
}

#[test]
fn messages_are_sent_and_received_server_clients() {
    initialize();

    let port = unused_port();
    let server_node = make_tcp_server_node(port);
    const CLIENT_COUNT: usize = 5;
    let client_nodes = make_client_nodes(port, CLIENT_COUNT as u8);
    wait();

    for client_node in client_nodes.values() {
        let message = minimal::messages::Heartbeat::default();
        client_node.send(&message).unwrap();
    }
    wait();

    for i in client_nodes.keys() {
        server_node.recv_frame().unwrap();
        log::info!("[server] received #{i}");
    }

    let message = minimal::messages::Heartbeat::default();
    server_node.send(&message).unwrap();
    wait_long();

    for (i, client_node) in client_nodes {
        client_node.try_recv_frame().unwrap();
        log::info!("[client] received: #{i}");
    }
}

#[test]
fn messages_are_sent_and_received_clients_server() {
    initialize();

    let port = unused_port();
    let server_node = make_tcp_server_node(port);
    const CLIENT_COUNT: usize = 5;
    let client_nodes = make_client_nodes(port, CLIENT_COUNT as u8);
    wait();

    for client_node in client_nodes.values() {
        let message = minimal::messages::Heartbeat::default();
        client_node.send(&message).unwrap();
    }
    wait_long();

    for i in client_nodes.keys() {
        server_node.try_recv_frame().unwrap();
        log::info!("[server] received #{i}");
    }
}

// #[test]
// fn closed_connections_are_dropped() {
//     initialize();
//
//     let port = unused_port();
//     let server_node = make_tcp_server_node(port);
//     let client_node = make_tcp_client_node(port, 0);
//     wait();
//
//     client_node.close().unwrap();
//     wait();
//
//     let message = minimal::messages::Heartbeat::default();
//     server_node.send(message.into()).unwrap();
//     wait();
//
//     {
//         let n_con = server_node.info().n_conn().unwrap();
//         assert_eq!(n_con, 0);
//     }
// }

#[test]
fn events_are_received() {
    initialize();

    let port = unused_port();
    let server_node = Node::try_from(
        Node::builder()
            .system_id(1)
            .component_id(1)
            .dialect::<Minimal>()
            .version(V2)
            .connection(TcpServer::new(make_addr(port)).unwrap())
            .heartbeat_timeout(WAIT_DURATION),
    )
    .unwrap();

    let client_node = make_tcp_client_node(port, 10);
    wait();

    let message = minimal::messages::Heartbeat::default();
    client_node.send(&message).unwrap();
    wait_long();

    for _ in 0..2 {
        match server_node.try_recv_event().unwrap() {
            Event::NewPeer(_) => {}
            Event::Frame(_, _) => {}
            _ => panic!("Invalid event!"),
        }
    }

    wait_long();
    match server_node.try_recv_event().unwrap() {
        Event::PeerLost(_) => {}
        _ => panic!("Invalid event!"),
    }
}

#[test]
fn heartbeats_are_sent() {
    initialize();

    let port = unused_port();
    let server_node = Node::try_from(
        Node::builder()
            .system_id(1)
            .component_id(1)
            .dialect::<Minimal>()
            .version(V2)
            .connection(TcpServer::new(make_addr(port)).unwrap())
            .heartbeat_timeout(WAIT_DURATION.mul_f32(2.0))
            .heartbeat_interval(WAIT_DURATION),
    )
    .unwrap();
    server_node.activate().unwrap();

    let client_node = make_tcp_client_node(port, 10);
    wait_long();

    client_node.try_recv_frame().unwrap();
    wait_long();
    client_node.try_recv_frame().unwrap();
}

#[test]
fn node_no_id_no_dialect_no_version() {
    initialize();

    let port = unused_port();
    let server_node = make_tcp_server_node(port);
    let client_node =
        Node::try_from(Node::builder().connection(TcpClient::new(make_addr(port)).unwrap()))
            .unwrap();
    wait();

    server_node
        .send(&minimal::messages::Heartbeat::default())
        .unwrap();
    wait_long();

    client_node.try_recv_frame().unwrap();

    let sequence: u8 = 190;
    let system_id: u8 = 42;
    let component_id: u8 = 142;
    let frame = mavio::Frame::builder()
        .sequence(sequence)
        .system_id(system_id)
        .component_id(component_id)
        .version(V2)
        .message(&minimal::messages::Heartbeat::default())
        .unwrap()
        .versionless();

    for _ in 0..5 {
        client_node.proxy_frame(&frame).unwrap();
    }

    for _ in 0..5 {
        let (frame, _) = server_node.recv_frame().unwrap();
        assert_eq!(frame.sequence(), sequence);
        assert_eq!(frame.system_id(), system_id);
        assert_eq!(frame.component_id(), component_id);
    }
}

#[test]
fn node_no_id_no_version() {
    initialize();

    let port = unused_port();
    let server_node = make_tcp_server_node(port);
    let client_node = Node::try_from(
        Node::builder()
            .dialect::<Minimal>()
            .connection(TcpClient::new(make_addr(port)).unwrap()),
    )
    .unwrap();
    wait();

    server_node
        .send(&minimal::messages::Heartbeat::default())
        .unwrap();
    wait_long();

    client_node.try_recv_frame().unwrap();
}

#[test]
fn node_no_id() {
    initialize();

    let port = unused_port();
    let server_node = make_tcp_server_node(port);
    let client_node = Node::try_from(
        Node::builder()
            .dialect::<Minimal>()
            .version(V2)
            .connection(TcpClient::new(make_addr(port)).unwrap()),
    )
    .unwrap();
    wait();

    server_node
        .send(&minimal::messages::Heartbeat::default())
        .unwrap();
    wait();

    let (frame, _) = client_node.recv_frame().unwrap();
    frame.decode::<minimal::Minimal>().unwrap();
}

#[test]
fn node_no_version() {
    initialize();

    let port = unused_port();
    let server_node = make_tcp_server_node(port);
    let client_node = Node::try_from(
        Node::builder()
            .system_id(42)
            .component_id(142)
            .dialect::<Minimal>()
            .connection(TcpClient::new(make_addr(port)).unwrap()),
    )
    .unwrap();
    wait();

    client_node
        .send_versioned(&minimal::messages::Heartbeat::default(), V2)
        .unwrap();
    wait_long();

    let (frame, _) = server_node.try_recv_frame().unwrap();
    assert_eq!(frame.system_id(), 42);
    assert_eq!(frame.component_id(), 142);
    assert!(matches!(frame.version(), MavLinkVersion::V2));

    let message = frame.decode().unwrap();
    if let minimal::Minimal::Heartbeat(_) = message {
        // message is fine
    } else {
        panic!("Invalid message!")
    }
}
