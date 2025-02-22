use mavio::protocol::SignedLinkId;
use portpicker::{pick_unused_port, Port};

use maviola::protocol::dialects::minimal::messages::Heartbeat;

use maviola::prelude::*;
use maviola::sync::prelude::*;

fn port() -> Port {
    pick_unused_port().unwrap()
}

fn make_server(address: &str, link_id: SignedLinkId, key: &str) -> Result<EdgeNode<V2>> {
    let server = Node::sync()
        .id(MavLinkId::new(1, 0))
        .connection(TcpServer::new(address)?)
        .signer(FrameSigner::new(link_id, key))
        .build()?;
    log::warn!("[server] started");
    Ok(server)
}

fn make_unauthorized_client(address: &str) -> Result<EdgeNode<V2>> {
    let unauthorized_client = Node::sync()
        .id(MavLinkId::new(1, 1))
        .connection(TcpClient::new(address)?)
        .build()?;
    log::warn!("[unauthorized_client] started");
    Ok(unauthorized_client)
}

fn make_authorized_client(address: &str, link_id: SignedLinkId, key: &str) -> Result<EdgeNode<V2>> {
    let authorised_client = Node::sync()
        .id(MavLinkId::new(1, 2))
        .connection(TcpClient::new(address)?)
        .signer(FrameSigner::new(link_id, key))
        .build()?;
    log::warn!("[authorised_client] started");
    Ok(authorised_client)
}

fn server_receive_unsigned_and_respond_signed(server: EdgeNode<V2>) -> Result<()> {
    for event in server.events() {
        match event {
            Event::Frame(frame, callback) => {
                assert!(
                    frame.is_signed(),
                    "frame should be signed by server upon receiving"
                );
                log::info!(
                    "[server] received signed frame with link ID: {}",
                    frame.link_id().unwrap()
                );
                callback.broadcast(&frame).unwrap();
                log::info!("[server] respond with signed frames to everyone but sender");
                break;
            }
            _ => {}
        }
    }

    Ok(())
}

fn authorized_client_receive_signed(authorized_client: EdgeNode<V2>) -> Result<()> {
    for event in authorized_client.events() {
        match event {
            Event::Frame(frame, _) => {
                assert!(
                    frame.is_signed(),
                    "authorized client should receive signed frame"
                );
                log::info!(
                    "[authorised_client] received signed frame with link ID: {}",
                    frame.link_id().unwrap()
                );
                break;
            }
            _ => {}
        }
    }

    Ok(())
}

fn run() -> Result<()> {
    let addr = format!("127.0.0.1:{}", port());
    let link_id = 1;
    let key = "something unsecure";

    let server = make_server(addr.as_str(), link_id, key)?;
    let unauthorized_client = make_unauthorized_client(addr.as_str())?;
    let authorized_client = make_authorized_client(addr.as_str(), link_id, key)?;

    unauthorized_client.send(&Heartbeat::default()).unwrap();
    log::info!("[unauthorized_client] send unsigned frame");

    server_receive_unsigned_and_respond_signed(server)?;
    authorized_client_receive_signed(authorized_client)?;

    log::warn!("[all] finished");
    Ok(())
}

fn main() {
    // Setup logger
    env_logger::builder()
        .filter_level(log::LevelFilter::Info) // Suppress everything below `info` for third-party modules.
        .filter_module(env!("CARGO_PKG_NAME"), log::LevelFilter::Info) // Log level for current package
        .init();

    run().unwrap();
}

#[cfg(test)]
#[test]
fn message_signing() {
    use std::thread;
    use std::time::Duration;

    let handler = thread::spawn(move || {
        run().unwrap();
    });

    for _ in 0..10 {
        thread::sleep(Duration::from_millis(250));
        if handler.is_finished() {
            handler.join().unwrap();
            return;
        }
    }

    if !handler.is_finished() {
        panic!("[message_signing] test took too long")
    }
}
