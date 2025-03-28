use std::fs::remove_file;
use std::path::PathBuf;
use std::time::Duration;

use maviola::protocol::dialects::minimal as dialect;

use maviola::asnc::prelude::*;
use maviola::prelude::*;

const N_ITER: u16 = 100;

fn report_frame<V: MaybeVersioned>(frame: &Frame<V>) {
    log::info!(
        "[reader] incoming frame #{} from {}:{}",
        frame.sequence(),
        frame.system_id(),
        frame.component_id()
    )
}

async fn run(path: PathBuf) -> Result<()> {
    let writer = Node::asnc::<V2>()
        .system_id(17)
        .component_id(42)
        .connection(FileWriter::new(path.as_path())?)
        .build()
        .await?;

    log::warn!("[writer] started");
    for _ in 0..N_ITER {
        writer
            .send(&dialect::messages::Heartbeat::default())
            .unwrap();
    }
    drop(writer);
    log::warn!("[writer] finished");

    let reader = Node::asnc::<V2>()
        .system_id(17)
        .component_id(42)
        .connection(FileReader::new(path.as_path())?)
        .build()
        .await?;

    log::warn!("[reader] started");
    let mut events = reader.events().unwrap();
    while let Some(event) = events.next().await {
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

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // Setup logger
    env_logger::builder()
        .filter_level(log::LevelFilter::Info) // Suppress everything below `info` for third-party modules.
        .filter_module(env!("CARGO_PKG_NAME"), log::LevelFilter::Info) // Log level for current package
        .init();

    let path = PathBuf::from("/tmp/maviola.bin");
    if path.exists() {
        remove_file(path.as_path()).unwrap();
    }
    run(path).await.unwrap();
}

#[cfg(test)]
#[tokio::test]
async fn async_file_rw() {
    let path = PathBuf::from("/tmp/maviola_async_file_rw.bin");
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
        panic!("[async_file_rw] test took too long")
    }
}
