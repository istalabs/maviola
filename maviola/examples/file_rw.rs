use std::fs::remove_file;
use std::path::PathBuf;
use std::sync::atomic;
use std::sync::atomic::AtomicU8;
use std::thread;
use std::time::Duration;

use maviola::dialects::minimal as dialect;

use maviola::prelude::*;
use maviola::sync::prelude::*;

const HEARTBEAT_TIMEOUT: Duration = Duration::from_millis(75);
const N_ITER: u16 = 100;

fn wait() {
    thread::sleep(Duration::from_millis(100));
}

fn report_frame<V: MaybeVersioned>(frame: &Frame<V>) {
    log::info!(
        "[reader] incoming frame #{} from {}:{}",
        frame.sequence(),
        frame.system_id(),
        frame.component_id()
    )
}

static SEQUENCE: AtomicU8 = AtomicU8::new(0);

fn make_frame(i: u16) -> Frame<V2> {
    let bytes: [u8; 2] = i.to_le_bytes();
    let (system_id, component_id) = (bytes[1] % 10, bytes[0] % 10);
    let sequence = SEQUENCE.fetch_add(1, atomic::Ordering::Release);

    let message = dialect::messages::Heartbeat::default();

    Frame::builder()
        .sequence(sequence)
        .system_id(system_id)
        .component_id(component_id)
        .version(V2)
        .message(&message)
        .unwrap()
        .build()
}

fn run(path: PathBuf) {
    let writer = Node::try_from(
        Node::builder()
            .system_id(17)
            .component_id(42)
            .version(V2)
            .dialect::<Minimal>()
            .connection(FileWriter::new(path.as_path()).unwrap()),
    )
    .unwrap();
    wait();

    for i in 0..N_ITER {
        let frame = make_frame(i);
        writer.proxy_frame(&frame).unwrap();
    }
    drop(writer);
    wait();

    let reader = Node::try_from(
        Node::builder()
            .system_id(17)
            .component_id(42)
            .version(V2)
            .dialect::<Minimal>()
            .heartbeat_timeout(HEARTBEAT_TIMEOUT)
            .connection(FileReader::new(path.as_path()).unwrap()),
    )
    .unwrap();

    for event in reader.events() {
        match event {
            Event::NewPeer(peer) => log::warn!("[server] new peer: {peer:?}"),
            Event::Frame(frame, _) => report_frame(&frame),
            Event::PeerLost(peer) => {
                log::warn!("[server] disconnected: {peer:?}");
                if !reader.has_peers() {
                    log::warn!("[server] all peers disconnected, exiting");
                    break;
                }
            }
        }
    }

    wait();
}

fn main() {
    // Setup logger
    env_logger::builder()
        .filter_level(log::LevelFilter::Info) // Suppress everything below `info` for third-party modules.
        .filter_module(env!("CARGO_PKG_NAME"), log::LevelFilter::Info) // Log level for current package
        .init();

    let path = PathBuf::from("/tmp/maviola.bin");
    if path.exists() {
        remove_file(path.as_path()).unwrap();
    }
    run(path);
}

#[cfg(test)]
#[test]
fn file_rw() {
    let path = PathBuf::from("/tmp/maviola_file_rw.bin");
    if path.exists() {
        remove_file(path.as_path()).unwrap();
    }
    let handler = thread::spawn(move || {
        run(path);
    });

    thread::sleep(Duration::from_secs(5));
    if !handler.is_finished() {
        panic!("[file_rw] test took too long")
    }
}
