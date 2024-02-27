use std::sync::atomic;
use std::sync::atomic::AtomicU8;
use std::time::Duration;

use maviola::asnc::io::{AsyncConnection, AsyncConnectionBuilder};
use maviola::asnc::io::{AsyncTcpClient, AsyncTcpServer};
use maviola::dialects::minimal as dialect;
use maviola::protocol::{Frame, MaybeVersioned, V2};

static SEQUENCE: AtomicU8 = AtomicU8::new(0);

fn report_frame<V: MaybeVersioned>(frame: &Frame<V>) {
    log::info!(
        "[server] incoming frame #{} from {}:{}",
        frame.sequence(),
        frame.system_id(),
        frame.component_id()
    )
}

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

async fn run() {
    let server = AsyncTcpServer::new("127.0.0.1:5600").unwrap();
    let mut server: AsyncConnection<V2> = server.build().await.unwrap();

    let client = AsyncTcpClient::new("127.0.0.1:5600").unwrap();
    let client: AsyncConnection<V2> = client.build().await.unwrap();

    tokio::task::spawn(async move {
        for i in 0..10 {
            client.send(&make_frame(i)).unwrap();
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    });

    for _ in 0..10 {
        match server.recv().await {
            Ok((frame, cb)) => {
                cb.respond(&frame).unwrap();
                report_frame(&frame);
            }
            Err(err) => {
                log::error!("[server] error: {err:?}");
                break;
            }
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // Setup logger
    env_logger::builder()
        .filter_level(log::LevelFilter::Info) // Suppress everything below `info` for third-party modules.
        .filter_module(env!("CARGO_PKG_NAME"), log::LevelFilter::Trace) // Allow everything from current package
        .init();

    run().await;
}
