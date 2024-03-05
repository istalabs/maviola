use std::thread;
use std::time::Duration;

use portpicker::{pick_unused_port, Port};

use maviola::protocol::ComponentId;

use maviola::prelude::*;
use maviola::sync::prelude::*;

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

    thread::spawn(move || -> Result<()> {
        let mut client = Node::builder()
            .version(V2)
            .system_id(31)
            .component_id(component_id)
            .heartbeat_interval(HEARTBEAT_INTERVAL)
            .heartbeat_timeout(HEARTBEAT_TIMEOUT)
            .connection(UdpClient::new(client_addr)?)
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

fn run(addr: &str) -> Result<()> {
    let server_addr = addr.to_string();
    let mut server = Node::builder()
        .version(V2)
        .dialect::<Minimal>()
        .system_id(17)
        .component_id(42)
        .heartbeat_interval(HEARTBEAT_INTERVAL)
        .heartbeat_timeout(HEARTBEAT_TIMEOUT)
        .connection(UdpServer::new(server_addr)?)
        .build()?;
    server.activate()?;

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

    Ok(())
}

fn main() {
    // Setup logger
    env_logger::builder()
        .filter_level(log::LevelFilter::Info) // Suppress everything below `info` for third-party modules.
        .filter_module(env!("CARGO_PKG_NAME"), log::LevelFilter::Info) // Log level for current package
        .init();

    let addr = addr(port());
    run(addr.as_str()).unwrap();
}

#[cfg(test)]
#[test]
fn udp_ping_pong() {
    let addr = addr(port());
    let handler = thread::spawn(move || {
        run(addr.as_str()).unwrap();
    });

    thread::sleep(Duration::from_secs(5));
    if !handler.is_finished() {
        panic!("[udp_ping_pong] test took too long")
    }
}
