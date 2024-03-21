use std::time::Duration;

use portpicker::{pick_unused_port, Port};

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
        let mut client = Node::asnc::<V2>()
            .system_id(31)
            .component_id(component_id)
            .heartbeat_interval(HEARTBEAT_INTERVAL)
            .heartbeat_timeout(HEARTBEAT_TIMEOUT)
            .connection(TcpClient::new(client_addr)?)
            .build()
            .await?;
        client.activate().await?;

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

        log::warn!("[{whoami}] finished");
        Ok::<(), Error>(())
    });
}

async fn run(addr: &str) -> Result<()> {
    let server_addr = addr.to_string();
    let mut server = Node::asnc::<V2>()
        .system_id(17)
        .component_id(42)
        .heartbeat_interval(HEARTBEAT_INTERVAL)
        .heartbeat_timeout(HEARTBEAT_TIMEOUT)
        .connection(TcpServer::new(server_addr)?)
        .build()
        .await?;
    server.activate().await?;

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
            _ => {}
        }
    }

    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // Setup logger
    env_logger::builder()
        .filter_level(log::LevelFilter::Info) // Suppress everything below `info` for third-party modules.
        .filter_module(env!("CARGO_PKG_NAME"), log::LevelFilter::Info) // Log level for current package
        .init();

    let addr = addr(port());
    run(addr.as_str()).await.unwrap();
}

#[cfg(test)]
#[tokio::test]
async fn async_tcp_ping_pong() {
    let addr = addr(port());
    let handler = tokio::spawn(async move {
        run(addr.as_str()).await.unwrap();
    });

    for _ in 0..10 {
        tokio::time::sleep(Duration::from_millis(250)).await;
        if handler.is_finished() {
            handler.await.unwrap();
            return;
        }
    }

    if !handler.is_finished() {
        panic!("[async_tcp_ping_pong] test took too long")
    }
}
