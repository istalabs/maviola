use std::collections::HashMap;
use std::sync::Once;
use std::thread;
use std::time::Duration;

use portpicker::Port;

use maviola::core::marker::Edge;
use maviola::dialects::minimal;
use maviola::protocol::{ComponentId, SystemId};
use maviola::sync::node::Event;

use maviola::prelude::*;
use maviola::sync::prelude::*;

static INIT: Once = Once::new();
static INIT_LOGGER: Once = Once::new();
pub const LOG_LEVEL: log::LevelFilter = log::LevelFilter::Debug;
pub const HOST: &str = "127.0.0.1";
const WAIT_DURATION: Duration = Duration::from_millis(50);
const WAIT_LONG_DURATION: Duration = Duration::from_millis(500);
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

pub fn make_tcp_server_node_v2(port: Port) -> Node<Edge<V2>, Minimal, V2, SyncApi<V2>> {
    Node::builder()
        .version(V2)
        .system_id(DEFAULT_TCP_SERVER_SYS_ID)
        .component_id(DEFAULT_TCP_SERVER_COMP_ID)
        .connection(TcpServer::new(make_addr(port)).unwrap())
        .build()
        .unwrap()
}

pub fn make_tcp_client_node_v2(
    port: Port,
    component_id: u8,
) -> Node<Edge<V2>, Minimal, V2, SyncApi<V2>> {
    Node::builder()
        .version(V2)
        .system_id(DEFAULT_TCP_CLIENT_SYS_ID)
        .component_id(component_id)
        .connection(TcpClient::new(make_addr(port)).unwrap())
        .build()
        .unwrap()
}

fn make_client_nodes_v2(port: Port, count: u8) -> HashMap<u8, EdgeNode<Minimal, V2>> {
    (0..count)
        .map(|i| (i, make_tcp_client_node_v2(port, i)))
        .collect()
}

#[test]
fn messages_are_sent_and_received_server_clients() {
    initialize();

    let port = unused_port();
    let server_node = make_tcp_server_node_v2(port);
    const CLIENT_COUNT: usize = 5;
    let client_nodes = make_client_nodes_v2(port, CLIENT_COUNT as u8);
    wait();

    for client_node in client_nodes.values() {
        let message = minimal::messages::Heartbeat::default();
        client_node.send(&message).unwrap();
    }
    wait();

    for _ in client_nodes.keys() {
        assert!(matches!(server_node.try_recv().unwrap(), Event::NewPeer(_)));
        assert!(matches!(
            server_node.try_recv().unwrap(),
            Event::Frame(_, _)
        ));
    }

    let message = minimal::messages::Heartbeat::default();
    server_node.send(&message).unwrap();
    wait_long();

    for client_node in client_nodes.values() {
        assert!(matches!(client_node.try_recv().unwrap(), Event::NewPeer(_)));
        assert!(matches!(
            client_node.try_recv().unwrap(),
            Event::Frame(_, _)
        ));
    }
}

#[test]
fn messages_are_sent_and_received_clients_server() {
    initialize();

    let port = unused_port();
    let server_node = make_tcp_server_node_v2(port);
    const CLIENT_COUNT: usize = 5;
    let client_nodes = make_client_nodes_v2(port, CLIENT_COUNT as u8);
    wait();

    for client_node in client_nodes.values() {
        let message = minimal::messages::Heartbeat::default();
        client_node.send(&message).unwrap();
    }
    wait_long();

    for _ in client_nodes.values() {
        assert!(matches!(server_node.try_recv().unwrap(), Event::NewPeer(_)));
        assert!(matches!(
            server_node.try_recv().unwrap(),
            Event::Frame(_, _)
        ));
    }
}

#[test]
fn events_are_received() {
    initialize();

    let port = unused_port();
    let server_node = Node::builder()
        .version(V2)
        .system_id(1)
        .component_id(1)
        .connection(TcpServer::new(make_addr(port)).unwrap())
        .heartbeat_timeout(WAIT_DURATION)
        .build()
        .unwrap();

    let client_node = make_tcp_client_node_v2(port, 10);
    wait();

    let message = minimal::messages::Heartbeat::default();
    client_node.send(&message).unwrap();
    wait_long();

    for _ in 0..2 {
        match server_node.try_recv().unwrap() {
            Event::NewPeer(_) => {}
            Event::Frame(_, _) => {}
            _ => panic!("Invalid event!"),
        }
    }

    wait_long();
    match server_node.try_recv().unwrap() {
        Event::PeerLost(_) => {}
        _ => panic!("Invalid event!"),
    }
}

#[test]
fn heartbeats_are_sent() {
    initialize();

    let port = unused_port();
    let mut server_node = Node::builder()
        .version(V2)
        .system_id(1)
        .component_id(1)
        .connection(TcpServer::new(make_addr(port)).unwrap())
        .heartbeat_timeout(WAIT_DURATION.mul_f32(2.0))
        .heartbeat_interval(WAIT_DURATION)
        .build()
        .unwrap();
    server_node.activate().unwrap();

    let client_node = make_tcp_client_node_v2(port, 10);
    wait_long();

    assert!(matches!(client_node.try_recv().unwrap(), Event::NewPeer(_)));
    assert!(matches!(
        client_node.try_recv().unwrap(),
        Event::Frame(_, _)
    ));
}

#[test]
fn node_no_id_no_dialect_no_version() {
    initialize();

    let port = unused_port();
    let server_node = make_tcp_server_node_v2(port);
    let client_node = Node::builder()
        .connection(TcpClient::new(make_addr(port)).unwrap())
        .build()
        .unwrap();
    wait();

    server_node
        .send(&minimal::messages::Heartbeat::default())
        .unwrap();
    wait_long();

    client_node.try_recv().unwrap();

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
        .build()
        .into_versionless();

    for _ in 0..5 {
        client_node.proxy_frame(&frame).unwrap();
    }

    wait_long();

    assert!(matches!(server_node.try_recv().unwrap(), Event::NewPeer(_)));

    for _ in 0..5 {
        if let Ok(Event::Frame(frame, _)) = server_node.try_recv() {
            assert_eq!(frame.sequence(), sequence);
            assert_eq!(frame.system_id(), system_id);
            assert_eq!(frame.component_id(), component_id);
        } else {
            panic!("invalid event!");
        }
    }
}

#[test]
fn node_no_id_no_version() {
    initialize();

    let port = unused_port();
    let server_node = make_tcp_server_node_v2(port);
    let client_node = Node::builder()
        .connection(TcpClient::new(make_addr(port)).unwrap())
        .build()
        .unwrap();
    wait();

    server_node
        .send(&minimal::messages::Heartbeat::default())
        .unwrap();
    wait_long();

    assert!(matches!(client_node.try_recv().unwrap(), Event::NewPeer(_)));
    assert!(matches!(
        client_node.try_recv().unwrap(),
        Event::Frame(_, _)
    ));
}

#[test]
fn node_no_id() {
    initialize();

    let port = unused_port();
    let server_node = make_tcp_server_node_v2(port);
    let client_node = Node::builder()
        .version(V2)
        .connection(TcpClient::new(make_addr(port)).unwrap())
        .build()
        .unwrap();
    wait();

    server_node
        .send(&minimal::messages::Heartbeat::default())
        .unwrap();
    wait_long();

    client_node.try_recv().unwrap();
    if let Event::Frame(frame, _) = client_node.recv().unwrap() {
        frame.decode::<Minimal>().unwrap();
    } else {
        panic!("invalid event!")
    }
}

#[test]
fn node_no_version() {
    initialize();

    let port = unused_port();
    let server_node = make_tcp_server_node_v2(port);
    let client_node = Node::builder()
        .system_id(42)
        .component_id(142)
        .dialect::<Minimal>()
        .connection(TcpClient::new(make_addr(port)).unwrap())
        .build()
        .unwrap();
    wait();

    client_node
        .send_versioned::<V2>(&minimal::messages::Heartbeat::default())
        .unwrap();
    wait_long();

    // Skip new peer event
    server_node.try_recv().unwrap();

    if let Event::Frame(frame, _) = server_node.try_recv().unwrap() {
        assert_eq!(frame.system_id(), 42);
        assert_eq!(frame.component_id(), 142);
        assert!(matches!(frame.version(), MavLinkVersion::V2));

        let message = frame.decode().unwrap();
        if let Minimal::Heartbeat(_) = message {
            // message is fine
        } else {
            panic!("invalid message!")
        }
    } else {
        panic!("invalid event!")
    }
}

#[test]
fn send_versionless_frames() {
    let port = unused_port();
    let node_v2 = make_tcp_server_node_v2(port);

    let frame_v2 = node_v2
        .next_frame(&minimal::messages::Heartbeat::default())
        .unwrap();
    node_v2
        .send_versionless_frame(&frame_v2.into_versionless())
        .unwrap();

    let frame_v1 = Frame::builder()
        .version(V1)
        .system_id(1)
        .component_id(1)
        .sequence(0)
        .message(&minimal::messages::Heartbeat::default())
        .unwrap()
        .build();
    let send_result = node_v2.send_versionless_frame(&frame_v1.to_versionless());
    assert!(send_result.is_err());
}
