use std::fs::remove_file;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use maviola::protocol::ComponentId;

use maviola::prelude::*;
use maviola::sync::prelude::*;

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

    thread::spawn(move || -> Result<()> {
        let mut client = Node::builder()
            .version(V2)
            .system_id(31)
            .component_id(component_id)
            .heartbeat_interval(HEARTBEAT_INTERVAL)
            .heartbeat_timeout(HEARTBEAT_TIMEOUT)
            .connection(SockClient::new(path)?)
            .build()?;
        client.activate()?;

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

        log::warn!("[{whoami}] finished");
        Ok(())
    });
}

fn run(path: PathBuf) -> Result<()> {
    let mut server = Node::builder()
        .version(V2)
        .system_id(17)
        .component_id(42)
        .heartbeat_interval(HEARTBEAT_INTERVAL)
        .heartbeat_timeout(HEARTBEAT_TIMEOUT)
        .connection(SockServer::new(path.as_path())?)
        .build()?;
    server.activate()?;
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

    Ok(())
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
    run(path).unwrap();
}

#[cfg(test)]
#[test]
fn tcp_ping_pong() {
    let path = PathBuf::from("/tmp/maviola_sock_ping_pong.sock");
    if path.exists() {
        remove_file(path.as_path()).unwrap();
    }
    let handler = thread::spawn(move || {
        run(path).unwrap();
    });

    thread::sleep(Duration::from_secs(5));
    if !handler.is_finished() {
        panic!("[sock_ping_pong] test took too long")
    }
}
