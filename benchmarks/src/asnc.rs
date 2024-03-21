use std::fs::remove_file;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use tokio::sync::Barrier;

use maviola::dialects::minimal::enums::{MavAutopilot, MavModeFlag, MavState, MavType};
use maviola::dialects::minimal::messages::Heartbeat;

use maviola::asnc::prelude::*;
use maviola::prelude::*;

const HEARTBEAT_INTERVAL: Duration = Duration::from_millis(50);
const HEARTBEAT_TIMEOUT: Duration = Duration::from_millis(75);
const WAIT_DURATION: Duration = Duration::from_millis(500);

async fn wait() {
    tokio::time::sleep(WAIT_DURATION).await;
}

async fn make_sock_server(path: PathBuf) -> EdgeNode<V2> {
    Node::builder()
        .version::<V2>()
        .system_id(1)
        .component_id(0)
        .heartbeat_interval(HEARTBEAT_INTERVAL)
        .heartbeat_timeout(HEARTBEAT_TIMEOUT)
        .async_connection(SockServer::new(path.as_path()).unwrap())
        .build()
        .await
        .unwrap()
}

async fn make_sock_client(path: PathBuf, id: u16) -> EdgeNode<V2> {
    let bytes: [u8; 2] = id.to_le_bytes();
    let system_id = bytes[0];
    let component_id = bytes[1];

    Node::builder()
        .version::<V2>()
        .system_id(system_id)
        .component_id(component_id)
        .heartbeat_interval(HEARTBEAT_INTERVAL)
        .heartbeat_timeout(HEARTBEAT_TIMEOUT)
        .async_connection(SockClient::new(path.as_path()).unwrap())
        .build()
        .await
        .unwrap()
}

pub async fn benchmark_async_unix_sockets(n_clients: u16, n_iter: usize) {
    let n_interaction = n_clients as u32 * n_iter as u32;
    let path = PathBuf::from("/tmp/maviola_async_benchmarks.sock");
    if Path::exists(path.as_path()) {
        remove_file(path.as_path()).unwrap();
    }
    let mut server = make_sock_server(path.clone()).await;
    wait().await;

    let barrier = Arc::new(Barrier::new(n_clients as usize + 1));

    for i in 0..n_clients {
        let path = path.clone();
        let barrier = barrier.clone();

        tokio::spawn(async move {
            barrier.wait().await;
            let client = make_sock_client(path, i).await;

            let message = Heartbeat {
                type_: MavType::Generic,
                autopilot: MavAutopilot::Generic,
                base_mode: MavModeFlag::all(),
                custom_mode: 0,
                system_status: MavState::Active,
                mavlink_version: DefaultDialect::version().unwrap(),
            };

            for _ in 0..n_iter {
                if let Err(err) = client.send(&message) {
                    log::error!("[client #{i}] send error: {err:?}");
                    break;
                }
            }
            barrier.wait().await;
        });
    }

    barrier.wait().await;

    let mut n_received_frames = 0;
    let modulo = n_interaction / 10;
    let timeout_per_frame = Duration::from_micros(100);

    log::info!("[benchmark_unix_sockets] started");

    let start = SystemTime::now();
    for _ in 0..n_interaction {
        match server.recv_frame().await {
            Ok(_) => {
                n_received_frames += 1;
                if n_received_frames % modulo == 0 {
                    let percents = 10 * n_received_frames / modulo;
                    log::info!("[server] {percents}%: {n_received_frames} frames");

                    let checkpoint = SystemTime::now();
                    let duration = checkpoint.duration_since(start).unwrap();
                    if duration > timeout_per_frame * n_received_frames {
                        log::error!("[server] stopping by timeout");
                        break;
                    }
                }
            }
            Err(err) => {
                log::error!("[server] error: {err:?}");
                break;
            }
        }
    }

    let end = SystemTime::now();
    let duration = end.duration_since(start).unwrap();

    drop(server);
    barrier.wait().await;
    wait().await;

    if n_received_frames < n_interaction {
        log::warn!(
            "[benchmark_unix_sockets] frame loss: {}%",
            (n_interaction - n_received_frames) as f32 / n_interaction as f32 * 100.0
        );
    }

    log::info!(
        "[benchmark_unix_sockets] receive {n_iter} frames from {n_clients} clients ({n_interaction} total): {}s, ({}ms per frame)",
        duration.as_secs_f32(),
        (duration.as_secs_f64() / n_received_frames as f64 * 1_000.0) as f32
    )
}
