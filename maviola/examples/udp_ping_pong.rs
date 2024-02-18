use mavio::Frame;
use std::thread;
use std::time::Duration;

use mavio::protocol::{ComponentId, MaybeVersioned, V2};
use portpicker::{pick_unused_port, Port};

use maviola::dialects::minimal as dialect;
use maviola::io::sync::{UdpClientConf, UdpServerConf};
use maviola::io::{Event, Node, NodeConf};

const HEARTBEAT_INTERVAL: Duration = Duration::from_millis(50);
const HEARTBEAT_TIMEOUT: Duration = Duration::from_millis(75);
const HOST: &str = "127.0.0.1";
const N_ITER: usize = 10;
const N_CLIENTS: ComponentId = 5;

fn port() -> Port {
    pick_unused_port().unwrap()
}

fn addr(port: Port) -> String {
    format!("{HOST}:{}", port)
}

fn report_frame<V: MaybeVersioned>(whoami: &str, frame: &Frame<V>) {
    log::info!(
        "[{whoami}] incoming frame #{} from {}:{}",
        frame.sequence(),
        frame.system_id(),
        frame.component_id()
    )
}

fn spawn_client(addr: &str, component_id: ComponentId) {
    let client_addr = addr.to_string();
    let whoami = format!("client #{component_id}");

    thread::spawn(move || {
        let client = Node::try_from(
            NodeConf::builder()
                .system_id(31)
                .component_id(component_id)
                .version(V2)
                .dialect(dialect::dialect())
                .heartbeat_interval(HEARTBEAT_INTERVAL)
                .heartbeat_timeout(HEARTBEAT_TIMEOUT)
                .connection(UdpClientConf::new(client_addr).unwrap())
                .build(),
        )
        .unwrap();
        client.activate().unwrap();

        log::warn!("[{whoami}] started as {:?}", client.info());

        let mut n_iter = 0;
        for event in client.events() {
            match event {
                Event::NewPeer(peer) => log::warn!("[{whoami}] new peer: {peer:?}"),
                Event::Frame(frame, _) => {
                    if n_iter == N_ITER {
                        break;
                    }
                    report_frame(whoami.as_str(), &frame);
                    n_iter += 1;
                }
                _ => {}
            }
        }

        log::warn!("[{whoami}] disconnected");
        drop(client);
    });
}

fn run(addr: &str) {
    let server_addr = addr.to_string();
    let server = Node::try_from(
        NodeConf::builder()
            .system_id(17)
            .component_id(42)
            .version(V2)
            .dialect(dialect::dialect())
            .heartbeat_interval(HEARTBEAT_INTERVAL)
            .heartbeat_timeout(HEARTBEAT_TIMEOUT)
            .connection(UdpServerConf::new(server_addr).unwrap())
            .build(),
    )
    .unwrap();
    server.activate().unwrap();

    for i in 0..N_CLIENTS {
        spawn_client(addr, i);
    }

    for event in server.events() {
        match event {
            Event::NewPeer(peer) => log::warn!("[server] new peer: {peer:?}"),
            Event::Frame(frame, _) => report_frame("server", &frame),
            Event::PeerLost(peer) => {
                log::warn!("[server] disconnected: {peer:?}");
                if !server.has_peers() {
                    log::warn!("[server] all peers disconnected, exiting");
                    break;
                }
            }
        }
    }
}

fn main() {
    // Setup logger
    env_logger::builder()
        .filter_level(log::LevelFilter::Info) // Suppress everything below `info` for third-party modules.
        .filter_module(env!("CARGO_PKG_NAME"), log::LevelFilter::Info) // Allow everything from current package
        .init();

    let addr = addr(port());
    run(addr.as_str());
}

#[cfg(test)]
#[test]
fn udp_ping_pong() {
    let addr = addr(port());
    let handler = thread::spawn(move || {
        run(addr.as_str());
    });

    thread::sleep(Duration::from_secs(5));
    if !handler.is_finished() {
        panic!("[udp_ping_pong] test took too long")
    }
}
