use std::time::Duration;

use portpicker::{pick_unused_port, Port};
use tokio_stream::StreamExt;

use maviola::protocol::ComponentId;

use maviola::asnc::prelude::*;
use maviola::prelude::*;

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

async fn spawn_client(addr: &str, component_id: ComponentId) {
    let client_addr = addr.to_string();
    let whoami = format!("client #{component_id}");

    tokio::spawn(async move {
        let mut client = Node::builder()
            .system_id(31)
            .component_id(component_id)
            .version(V2)
            .heartbeat_interval(HEARTBEAT_INTERVAL)
            .heartbeat_timeout(HEARTBEAT_TIMEOUT)
            .async_connection(TcpClient::new(client_addr).unwrap())
            .build()
            .await
            .unwrap();
        client.activate().await.unwrap();

        log::warn!("[{whoami}] started as {:?}", client.info());

        let mut n_iter = 0;
        let mut events = client.events().unwrap();
        while let Some(event) = events.next().await {
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

async fn run(addr: &str) {
    let server_addr = addr.to_string();
    let mut server = Node::builder()
        .system_id(17)
        .component_id(42)
        .version(V2)
        .dialect::<Minimal>()
        .heartbeat_interval(HEARTBEAT_INTERVAL)
        .heartbeat_timeout(HEARTBEAT_TIMEOUT)
        .async_connection(TcpServer::new(server_addr).unwrap())
        .build()
        .await
        .unwrap();
    server.activate().await.unwrap();

    log::warn!("[server] started as {:?}", server.info());

    for i in 0..N_CLIENTS {
        spawn_client(addr, i).await;
    }

    let mut events = server.events().unwrap();
    while let Some(event) = events.next().await {
        match event {
            Event::NewPeer(peer) => log::warn!("[server] new peer: {peer:?}"),
            Event::Frame(frame, _) => report_frame("server", &frame),
            Event::PeerLost(peer) => {
                log::warn!("[server] disconnected: {peer:?}");
                if !server.has_peers().await {
                    log::warn!("[server] all peers disconnected, exiting");
                    break;
                }
            }
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // Setup logger
    env_logger::builder()
        .filter_level(log::LevelFilter::Info) // Suppress everything below `info` for third-party modules.
        .filter_module(env!("CARGO_PKG_NAME"), log::LevelFilter::Info) // Log level for current package
        .init();

    let addr = addr(port());
    run(addr.as_str()).await;
}

#[cfg(test)]
#[tokio::test]
async fn tcp_ping_pong() {
    let addr = addr(port());
    let handler = tokio::spawn(async move {
        run(addr.as_str()).await;
    });

    tokio::time::sleep(Duration::from_secs(5)).await;
    if !handler.is_finished() {
        panic!("[async_tcp_ping_pong] test took too long")
    }
}
