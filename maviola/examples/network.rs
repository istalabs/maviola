use portpicker::{pick_unused_port, Port};
use std::thread;
use std::time::Duration;

use maviola::dialects::minimal::messages::Heartbeat;
use maviola::protocol::ComponentId;

use maviola::prelude::*;
use maviola::sync::prelude::*;

const HOST: &str = "127.0.0.1";
const WAIT_DURATION: Duration = Duration::from_millis(250);

fn wait() {
    thread::sleep(WAIT_DURATION);
}

fn port() -> Port {
    pick_unused_port().unwrap()
}

fn addr(port: Port) -> String {
    format!("{HOST}:{}", port)
}

fn make_network_node_server(addr_1: &str, addr_2: &str) -> Result<EdgeNode<V2>> {
    Node::builder()
        .version::<V2>()
        .id(MavLinkId::new(1, 0))
        .connection(
            Network::synchronous()
                .add_connection(TcpServer::new(addr_1)?)
                .add_connection(TcpServer::new(addr_2)?),
        )
        .build()
}

fn make_client(addr: &str, component_id: ComponentId) -> Result<EdgeNode<V2>> {
    Node::builder()
        .version::<V2>()
        .id(MavLinkId::new(1, component_id))
        .connection(TcpClient::new(addr)?)
        .build()
}

fn run(addr_1: &str, addr_2: &str) -> Result<()> {
    let server = make_network_node_server(addr_1, addr_2)?;
    wait();

    let client_1 = make_client(addr_1, 1)?;
    let client_2 = make_client(addr_2, 2)?;
    wait();

    // Send frame to the first address
    client_1.send(&Heartbeat::default())?;

    // Get frame from the first address
    let (frame, callback) = server.recv_frame()?;
    // Broadcast frame to everyone except sender
    callback.respond_others(&frame)?;
    wait();

    // Get frame at the second address
    let (frame, _) = client_2.recv_frame()?;
    // Make sure that frame is sent by the first client
    assert_eq!(frame.component_id(), 1);

    Ok(())
}

fn main() {
    // Setup logger
    env_logger::builder()
        .filter_level(log::LevelFilter::Info) // Suppress everything below `info` for third-party modules.
        .filter_module(env!("CARGO_PKG_NAME"), log::LevelFilter::Info) // Log level for current package
        .init();

    let addr_1 = addr(port());
    let addr_2 = addr(port());
    run(addr_1.as_str(), addr_2.as_str()).unwrap();
}

#[cfg(test)]
#[test]
fn network() {
    let addr_1 = addr(port());
    let addr_2 = addr(port());

    let handler = thread::spawn(move || {
        run(addr_1.as_str(), addr_2.as_str()).unwrap();
    });

    for _ in 0..10 {
        thread::sleep(Duration::from_millis(250));
        if handler.is_finished() {
            handler.join().unwrap();
            return;
        }
    }

    if !handler.is_finished() {
        panic!("[network] test took too long")
    }
}
