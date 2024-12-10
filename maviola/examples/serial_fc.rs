use std::thread;
use std::time::Duration;

use maviola::prelude::*;
use maviola::sync::prelude::*;

const N_ITER: u16 = 50;
#[cfg(target_os = "windows")]
const DEVICE_PREFIX: &str = "COM";
#[cfg(target_os = "macos")]
const DEVICE_PREFIX: &str = "/dev/tty.usbmodem";
#[cfg(target_os = "linux")]
const DEVICE_PREFIX: &str = "/dev/ttyACM";
#[cfg(all(not(target_os = "macos"), not(target_os = "linux"), unix))]
const DEVICE_PREFIX: &str = "/dev/tty";
const BAUD_RATE: u32 = 115_200;

fn wait() {
    thread::sleep(Duration::from_millis(100));
}

fn lookup() -> Option<String> {
    let mut path = None;
    let ports = serialport::available_ports().unwrap();
    log::warn!(
        "Got ports: {:?}",
        ports
            .iter()
            .map(|p| p.port_name.clone())
            .collect::<Vec<_>>()
    );
    for port in ports {
        if port.port_name.starts_with(DEVICE_PREFIX) {
            path = Some(port.port_name);
            break;
        }
    }
    path
}

fn report_frame<V: MaybeVersioned>(frame: &Frame<V>) -> Result<()> {
    let msg = frame.decode::<DefaultDialect>()?;
    log::info!(
        "[serial] incoming frame #{} from {}:{}:\n{:?}",
        frame.sequence(),
        frame.system_id(),
        frame.component_id(),
        msg
    );
    Ok(())
}

fn run(path: &String) -> Result<()> {
    log::warn!("[serial] connecting to {}", path);

    let serial = Node::sync::<V2>()
        .system_id(17)
        .component_id(42)
        .connection(SerialPort::new(path, BAUD_RATE)?)
        .build()?;

    log::warn!("[serial] heartbeats sequence started");
    wait();
    for _ in 0..N_ITER {
        serial.send(&default_dialect::messages::Heartbeat::default())?;
    }
    wait();
    log::warn!("[serial] heartbeats sequence finished");

    log::warn!("[serial] listening for the next {} frames", N_ITER);
    let mut n_iter = 0;
    for event in serial.events() {
        if n_iter == N_ITER {
            break;
        }
        n_iter += 1;

        match event {
            Event::NewPeer(peer) => log::warn!("[serial] new peer: {peer:?}"),
            Event::Frame(frame, _) => report_frame(&frame)?,
            Event::PeerLost(peer) => {
                log::warn!("[serial] disconnected: {peer:?}");
                break;
            }
            _ => {}
        }
    }
    log::warn!("[serial] finished");

    Ok(())
}

fn main() {
    // Setup logger
    env_logger::builder()
        .filter_level(log::LevelFilter::Info) // Suppress everything below `info` for third-party modules.
        .filter_module(env!("CARGO_PKG_NAME"), log::LevelFilter::Info) // Log level for current package
        .init();

    run(&lookup().expect("No available ports")).unwrap();
}
