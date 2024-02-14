use std::collections::HashMap;
use std::sync::Once;
use std::thread;
use std::time::Duration;

use mavio::protocol::{MavLinkVersion, V2};

use maviola::dialects::minimal;
use maviola::io::sync::{TcpClientConf, TcpServerConf};
use maviola::io::NodeConf;
use maviola::io::{Event, Node};
use maviola::marker::{HasDialect, Identified};

static INIT: Once = Once::new();
const LOG_LEVEL: log::LevelFilter = log::LevelFilter::Debug;
const WAIT_DURATION: Duration = Duration::from_millis(50);
const WAIT_LONG_DURATION: Duration = Duration::from_millis(500);
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

fn wait_long() {
    thread::sleep(WAIT_LONG_DURATION)
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

fn make_server_node(port: portpicker::Port) -> Node<Identified, HasDialect<minimal::Minimal>, V2> {
    Node::try_from(
        NodeConf::builder()
            .system_id(1)
            .component_id(1)
            .dialect(minimal::dialect())
            .version(V2)
            .conn_conf(TcpServerConf::new(make_addr(port)).unwrap())
            .build(),
    )
    .unwrap()
}

fn make_client_node(
    port: portpicker::Port,
    component_id: u8,
) -> Node<Identified, HasDialect<minimal::Minimal>, V2> {
    Node::try_from(
        NodeConf::builder()
            .system_id(2)
            .component_id(component_id)
            .dialect(minimal::dialect())
            .version(V2)
            .conn_conf(TcpClientConf::new(make_addr(port)).unwrap())
            .build(),
    )
    .unwrap()
}

fn make_client_nodes(
    port: portpicker::Port,
    count: u8,
) -> HashMap<u8, Node<Identified, HasDialect<minimal::Minimal>, V2>> {
    (0..count).map(|i| (i, make_client_node(port, i))).collect()
}

#[test]
fn messages_are_sent_and_received_server_clients() {
    initialize();

    let port = unused_port();
    let server_node = make_server_node(port);
    const CLIENT_COUNT: usize = 5;
    let client_nodes = make_client_nodes(port, CLIENT_COUNT as u8);
    wait();

    for client_node in client_nodes.values() {
        let message = minimal::messages::Heartbeat::default();
        client_node.send(message.into()).unwrap();
    }
    wait();

    for i in client_nodes.keys() {
        server_node.recv_frame().unwrap();
        log::info!("[server] received #{i}");
    }

    let message = minimal::messages::Heartbeat::default();
    server_node.send(message.into()).unwrap();
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
    let server_node = make_server_node(port);
    const CLIENT_COUNT: usize = 5;
    let client_nodes = make_client_nodes(port, CLIENT_COUNT as u8);
    wait();

    for client_node in client_nodes.values() {
        let message = minimal::messages::Heartbeat::default();
        client_node.send(message.into()).unwrap();
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
//     let server_node = make_server_node(port);
//     let client_node = make_client_node(port, 0);
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
        NodeConf::builder()
            .system_id(1)
            .component_id(1)
            .dialect(minimal::dialect())
            .version(V2)
            .conn_conf(TcpServerConf::new(make_addr(port)).unwrap())
            .timeout(WAIT_DURATION)
            .build(),
    )
    .unwrap();

    let client_node = make_client_node(port, 10);
    wait();

    let message = minimal::messages::Heartbeat::default();
    client_node.send(message.into()).unwrap();
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
        NodeConf::builder()
            .system_id(1)
            .component_id(1)
            .dialect(minimal::dialect())
            .version(V2)
            .conn_conf(TcpServerConf::new(make_addr(port)).unwrap())
            .timeout(WAIT_DURATION)
            .build(),
    )
    .unwrap();
    server_node.start().unwrap();

    let client_node = make_client_node(port, 10);
    wait_long();

    client_node.try_recv_frame().unwrap();
    wait_long();
    client_node.try_recv_frame().unwrap();
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
    wait_long();

    client_node.try_recv_frame().unwrap();
}

#[test]
fn node_no_id() {
    initialize();

    let port = unused_port();
    let server_node = make_server_node(port);
    let client_node = Node::try_from(
        NodeConf::builder()
            .dialect(minimal::dialect())
            .version(V2)
            .conn_conf(TcpClientConf::new(make_addr(port)).unwrap())
            .build(),
    )
    .unwrap();
    wait();

    server_node
        .send(minimal::messages::Heartbeat::default().into())
        .unwrap();
    wait();

    let (frame, _) = client_node.recv_frame().unwrap();
    frame.decode::<minimal::Minimal>().unwrap();
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
            .dialect(minimal::dialect())
            .conn_conf(TcpClientConf::new(make_addr(port)).unwrap())
            .build(),
    )
    .unwrap();
    wait();

    client_node
        .send_versioned(minimal::messages::Heartbeat::default().into(), V2)
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
