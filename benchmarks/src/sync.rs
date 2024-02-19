use std::fs::remove_file;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, SystemTime};

use maviola::dialects::minimal;
use maviola::dialects::minimal::enums::{MavAutopilot, MavModeFlag, MavState, MavType};
use maviola::protocol::{HasDialect, Identified, V2};
use maviola::{Node, NodeConf, SockClientConf, SockServerConf};

const HEARTBEAT_INTERVAL: Duration = Duration::from_millis(50);
const HEARTBEAT_TIMEOUT: Duration = Duration::from_millis(75);
const WAIT_DURATION: Duration = Duration::from_millis(100);

fn wait() {
    thread::sleep(WAIT_DURATION);
}

fn make_sock_server(path: PathBuf) -> Node<Identified, HasDialect<minimal::Minimal>, V2> {
    Node::try_from(
        NodeConf::builder()
            .system_id(1)
            .component_id(0)
            .version(V2)
            .dialect(minimal::dialect())
            .heartbeat_interval(HEARTBEAT_INTERVAL)
            .heartbeat_timeout(HEARTBEAT_TIMEOUT)
            .connection(SockServerConf::new(path.as_path()).unwrap())
            .build(),
    )
    .unwrap()
}

fn make_sock_client(path: PathBuf, id: u16) -> Node<Identified, HasDialect<minimal::Minimal>, V2> {
    let bytes: [u8; 2] = id.to_le_bytes();
    let system_id = bytes[0];
    let component_id = bytes[1];

    Node::try_from(
        NodeConf::builder()
            .system_id(system_id)
            .component_id(component_id)
            .version(V2)
            .dialect(minimal::dialect())
            .heartbeat_interval(HEARTBEAT_INTERVAL)
            .heartbeat_timeout(HEARTBEAT_TIMEOUT)
            .connection(SockClientConf::new(path.as_path()).unwrap())
            .build(),
    )
    .unwrap()
}

pub fn benchmark_unix_sockets(n_clients: u16, n_inter: usize) {
    let path = PathBuf::from("/tmp/maviola_benchmarks.sock");
    if Path::exists(path.as_path()) {
        remove_file(path.as_path()).unwrap();
    }
    let server = make_sock_server(path.clone());
    server.activate().unwrap();
    wait();

    for i in 0..n_clients {
        let client = make_sock_client(path.clone(), i);
        client.activate().unwrap();

        thread::spawn(move || {
            let message = minimal::messages::Heartbeat {
                type_: MavType::Generic,
                autopilot: MavAutopilot::Generic,
                base_mode: MavModeFlag::all(),
                custom_mode: 0,
                system_status: MavState::Active,
                mavlink_version: minimal::spec().version().unwrap(),
            };

            for _ in 0..n_inter {
                if let Err(err) = client.send(message.clone().into()) {
                    log::debug!("[client #{i}] send error: {err:?}");
                    return;
                }
            }
        });
    }

    let start = SystemTime::now();
    for _ in 0..n_inter {
        match server.recv_frame() {
            Ok((frame, res)) => res.respond(&frame).unwrap(),
            Err(_) => break,
        }
    }
    let end = SystemTime::now();
    let duration = end.duration_since(start).unwrap();

    log::info!(
        "[benchmark_unix_sockets] {n_inter} interactions with {n_clients} clients: {}s, ({}ms per interaction)",
        duration.as_secs_f32(),
        (duration.as_secs_f64() / n_inter as f64 * 1_000.0) as f32
    )
}
