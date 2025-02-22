use std::fs::remove_file;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use maviola::protocol::dialects::minimal as dialect;

use maviola::prelude::*;
use maviola::sync::prelude::*;

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

fn run(path: PathBuf) -> Result<()> {
    let writer = Node::sync::<V2>()
        .system_id(17)
        .component_id(42)
        .connection(FileWriter::new(path.as_path())?)
        .build()?;

    log::warn!("[writer] started");
    wait();
    for _ in 0..N_ITER {
        writer.send(&dialect::messages::Heartbeat::default())?;
    }
    drop(writer);
    wait();
    log::warn!("[writer] finished");

    let reader = Node::sync::<V2>()
        .system_id(17)
        .component_id(42)
        .connection(FileReader::new(path.as_path())?)
        .build()?;

    log::warn!("[reader] started");
    for event in reader.events() {
        match event {
            Event::NewPeer(peer) => log::warn!("[reader] new peer: {peer:?}"),
            Event::Frame(frame, _) => report_frame(&frame),
            Event::PeerLost(peer) => {
                log::warn!("[reader] disconnected: {peer:?}");
            }
            _ => {}
        }
    }
    log::warn!("[reader] finished");

    Ok(())
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
    run(path).unwrap();
}

#[cfg(test)]
#[test]
fn file_rw() {
    let path = PathBuf::from("/tmp/maviola_file_rw.bin");
    if path.exists() {
        remove_file(path.as_path()).unwrap();
    }
    let handler = thread::spawn(move || {
        run(path).unwrap();
    });

    for _ in 0..10 {
        thread::sleep(Duration::from_millis(250));
        if handler.is_finished() {
            handler.join().unwrap();
            return;
        }
    }

    if !handler.is_finished() {
        panic!("[file_rw] test took too long")
    }
}
