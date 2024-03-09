use portpicker::{pick_unused_port, Port};
use std::time::Duration;

use maviola::dialects::minimal::messages::Heartbeat;
use maviola::protocol::ComponentId;

use maviola::asnc::prelude::*;
use maviola::prelude::*;

const HOST: &str = "127.0.0.1";
const WAIT_DURATION: Duration = Duration::from_millis(250);

async fn wait() {
    tokio::time::sleep(WAIT_DURATION).await;
}

fn port() -> Port {
    pick_unused_port().unwrap()
}

fn addr(port: Port) -> String {
    format!("{HOST}:{}", port)
}

async fn make_network_node_server(addr_1: &str, addr_2: &str) -> Result<EdgeNode<V2>> {
    Node::builder()
        .version::<V2>()
        .id(MavLinkId::new(1, 0))
        .async_connection(
            Network::asynchronous()
                .add_connection(TcpServer::new(addr_1)?)
                .add_connection(TcpServer::new(addr_2)?),
        )
        .build()
        .await
}

async fn make_client(addr: &str, component_id: ComponentId) -> Result<EdgeNode<V2>> {
    Node::builder()
        .version::<V2>()
        .id(MavLinkId::new(1, component_id))
        .async_connection(TcpClient::new(addr)?)
        .build()
        .await
}

async fn run(addr_1: &str, addr_2: &str) -> Result<()> {
    let mut server = make_network_node_server(addr_1, addr_2).await?;
    wait().await;

    let client_1 = make_client(addr_1, 1).await?;
    let mut client_2 = make_client(addr_2, 2).await?;
    wait().await;

    // Send frame to the first address
    client_1.send(&Heartbeat::default())?;

    // Get frame from the first address
    let (frame, callback) = server.recv_frame().await?;
    // Broadcast frame to everyone except sender
    callback.respond_others(&frame)?;
    wait().await;

    // Get frame at the second address
    let (frame, _) = client_2.recv_frame().await?;
    // Make sure that frame is sent by the first client
    assert_eq!(frame.component_id(), 1);

    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // Setup logger
    env_logger::builder()
        .filter_level(log::LevelFilter::Info) // Suppress everything below `info` for third-party modules.
        .filter_module(env!("CARGO_PKG_NAME"), log::LevelFilter::Info) // Log level for current package
        .init();

    let addr_1 = addr(port());
    let addr_2 = addr(port());
    run(addr_1.as_str(), addr_2.as_str()).await.unwrap();
}

#[cfg(test)]
#[tokio::test]
async fn network() {
    let addr_1 = addr(port());
    let addr_2 = addr(port());

    let handler = tokio::spawn(async move {
        run(addr_1.as_str(), addr_2.as_str()).await.unwrap();
    });

    for _ in 0..10 {
        tokio::time::sleep(Duration::from_millis(250)).await;
        if handler.is_finished() {
            handler.await.unwrap();
            return;
        }
    }

    if !handler.is_finished() {
        panic!("[async_network] test took too long")
    }
}
