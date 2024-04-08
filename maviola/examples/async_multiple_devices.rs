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

async fn make_server_proxy_node(addr: &str) -> Result<ProxyNode<V2>> {
    Node::asnc().connection(TcpServer::new(addr)?).build().await
}

fn make_device_node(component_id: ComponentId, node: &ProxyNode<V2>) -> EdgeNode<V2> {
    Node::asnc()
        .id(MavLinkId::new(0, component_id))
        .build_from(node)
}

async fn make_client(addr: &str, component_id: ComponentId) -> Result<EdgeNode<V2>> {
    Node::asnc()
        .id(MavLinkId::new(1, component_id))
        .connection(TcpClient::new(addr)?)
        .build()
        .await
}

async fn run(addr: &str) -> Result<()> {
    // Create a proxy node that will hold the connection
    let server_proxy_node = make_server_proxy_node(addr).await?;
    wait().await;

    // Create devices from proxy node that will reuse the main node connection
    let device_1 = make_device_node(1, &server_proxy_node);
    let device_2 = make_device_node(2, &server_proxy_node);

    let mut client = make_client(addr, 1).await?;
    // We need to wait a bit otherwise client won't be able to pick up the message
    // (not required for real-life use cases)
    wait().await;

    // Send frame from the first device
    device_1.send(&Heartbeat::default())?;

    // Get frame from the first device
    let (frame, _) = client.recv_frame_timeout(WAIT_DURATION).await?;
    assert_eq!(frame.component_id(), 1);
    log::info!("[async_multiple_devices] received a frame from the first device");

    // Dropping a device won't affect the connection:
    drop(device_1);

    // Send frame from the second device
    device_2.send(&Heartbeat::default())?;

    // Get frame from the second device
    let (frame, _) = client.recv_frame_timeout(WAIT_DURATION).await?;
    assert_eq!(frame.component_id(), 2);
    log::info!("[async_multiple_devices] received a frame from the second device");

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
async fn async_multiple_devices() {
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
        panic!("[async_multiple_devices] test took too long")
    }
}
