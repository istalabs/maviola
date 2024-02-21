use std::fs::remove_file;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use maviola::dialects::minimal as dialect;
use maviola::io::{Event, Node, NodeConf};
use maviola::protocol::{ComponentId, Frame, MaybeVersioned, V2};
use maviola::{SockClientConf, SockServerConf};

const HEARTBEAT_INTERVAL: Duration = Duration::from_millis(50);
const HEARTBEAT_TIMEOUT: Duration = Duration::from_millis(75);
const N_ITER: usize = 10;
const N_CLIENTS: ComponentId = 5;

fn wait() {
    thread::sleep(Duration::from_millis(100));
}

fn report_frame<V: MaybeVersioned>(whoami: &str, frame: &Frame<V>) {
    log::info!(
        "[{whoami}] incoming frame #{} from {}:{}",
        frame.sequence(),
        frame.system_id(),
        frame.component_id()
    )
}

fn spawn_client(path: PathBuf, component_id: ComponentId) {
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
                .connection(SockClientConf::new(path).unwrap())
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

fn run(path: PathBuf) {
    let server = Node::try_from(
        NodeConf::builder()
            .system_id(17)
            .component_id(42)
            .version(V2)
            .dialect(dialect::dialect())
            .heartbeat_interval(HEARTBEAT_INTERVAL)
            .heartbeat_timeout(HEARTBEAT_TIMEOUT)
            .connection(SockServerConf::new(path.as_path()).unwrap())
            .build(),
    )
    .unwrap();
    server.activate().unwrap();
    wait();

    for i in 0..N_CLIENTS {
        let path = path.clone();
        spawn_client(path, i);
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
        .filter_module(env!("CARGO_PKG_NAME"), log::LevelFilter::Info) // Log level for current package
        .init();

    let path = PathBuf::from("/tmp/maviola.sock");
    if path.exists() {
        remove_file(path.as_path()).unwrap();
    }
    run(path);
}

#[cfg(test)]
#[test]
fn tcp_ping_pong() {
    let path = PathBuf::from("/tmp/maviola_sock_ping_pong.sock");
    if path.exists() {
        remove_file(path.as_path()).unwrap();
    }
    let handler = thread::spawn(move || {
        run(path);
    });

    thread::sleep(Duration::from_secs(5));
    if !handler.is_finished() {
        panic!("[sock_ping_pong] test took too long")
    }
}
