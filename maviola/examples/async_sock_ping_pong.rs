use std::fs::remove_file;
use std::path::PathBuf;
use std::time::Duration;

use maviola::protocol::ComponentId;

use maviola::asnc::prelude::*;
use maviola::prelude::*;

const HEARTBEAT_INTERVAL: Duration = Duration::from_millis(50);
const HEARTBEAT_TIMEOUT: Duration = Duration::from_millis(75);
const N_ITER: usize = 10;
const N_CLIENTS: ComponentId = 5;

async fn wait() {
    tokio::time::sleep(Duration::from_millis(100)).await;
}

fn report_frame<V: MaybeVersioned>(whoami: &str, frame: &Frame<V>) {
    log::info!(
        "[{whoami}] incoming frame #{} from {}:{}",
        frame.sequence(),
        frame.system_id(),
        frame.component_id()
    )
}

async fn spawn_client(path: PathBuf, component_id: ComponentId) {
    let whoami = format!("client #{component_id}");

    tokio::spawn(async move {
        let mut client = Node::asnc::<V2>()
            .system_id(31)
            .component_id(component_id)
            .heartbeat_interval(HEARTBEAT_INTERVAL)
            .heartbeat_timeout(HEARTBEAT_TIMEOUT)
            .connection(SockClient::new(path)?)
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

async fn run(path: PathBuf) -> Result<()> {
    let mut server = Node::asnc::<V2>()
        .system_id(17)
        .component_id(42)
        .heartbeat_interval(HEARTBEAT_INTERVAL)
        .heartbeat_timeout(HEARTBEAT_TIMEOUT)
        .connection(SockServer::new(path.as_path())?)
        .build()
        .await?;
    server.activate().await?;
    wait().await;

    for i in 0..N_CLIENTS {
        let path = path.clone();
        spawn_client(path, i).await;
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

    let path = PathBuf::from("/tmp/maviola.sock");
    if path.exists() {
        remove_file(path.as_path()).unwrap();
    }
    run(path).await.unwrap();
}

#[cfg(test)]
#[tokio::test]
async fn async_sock_ping_pong() {
    let path = PathBuf::from("/tmp/maviola_async_sock_ping_pong.sock");
    if path.exists() {
        remove_file(path.as_path()).unwrap();
    }
    let handler = tokio::spawn(async move {
        run(path).await.unwrap();
    });

    for _ in 0..10 {
        tokio::time::sleep(Duration::from_millis(250)).await;
        if handler.is_finished() {
            handler.await.unwrap();
            return;
        }
    }

    if !handler.is_finished() {
        panic!("[async_sock_ping_pong] test took too long")
    }
}
