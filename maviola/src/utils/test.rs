//! Maviola test utils.
//!
//! These utils are generated only when `#[cfg(test)]` enabled.

use portpicker::Port;
use std::sync::Once;
use std::thread;
use std::time::Duration;

use mavio::dialects::minimal;
use mavio::protocol::V2;
use mavio::protocol::{ComponentId, SystemId};

use crate::marker::{HasDialect, Identified};
use crate::{Node, NodeConf, TcpClientConf, TcpServerConf, UdpClientConf, UdpServerConf};

static INIT_LOGGER: Once = Once::new();
pub const HOST: &str = "127.0.0.1";
pub const LOG_LEVEL: log::LevelFilter = log::LevelFilter::Debug;
pub const WAIT_DURATION: Duration = Duration::from_micros(100);
pub const WAIT_LONG_DURATION: Duration = Duration::from_micros(1000);
pub const DEFAULT_TCP_SERVER_SYS_ID: SystemId = 2;
pub const DEFAULT_TCP_SERVER_COMP_ID: ComponentId = 0;
pub const DEFAULT_TCP_CLIENT_SYS_ID: SystemId = 20;
pub const DEFAULT_UDP_SERVER_SYS_ID: SystemId = 3;
pub const DEFAULT_UDP_SERVER_COMP_ID: ComponentId = 0;
pub const DEFAULT_UDP_CLIENT_SYS_ID: SystemId = 30;

pub fn init_logger() {
    INIT_LOGGER.call_once(|| {
        env_logger::builder()
            // Suppress everything below `warn` for third-party modules
            .filter_level(log::LevelFilter::Warn)
            // Allow everything above `LOG_LEVEL` from current package
            .filter_module(env!("CARGO_PKG_NAME"), LOG_LEVEL)
            .init();
    });
}

pub fn wait() {
    thread::sleep(WAIT_DURATION)
}

pub fn wait_long() {
    thread::sleep(WAIT_LONG_DURATION)
}

pub fn unused_port() -> Port {
    portpicker::pick_unused_port().unwrap()
}

pub fn make_addr(port: Port) -> String {
    format!("{HOST}:{port}")
}

pub fn make_tcp_server_node(port: Port) -> Node<Identified, HasDialect<minimal::Minimal>, V2> {
    Node::try_from(
        NodeConf::builder()
            .system_id(DEFAULT_TCP_SERVER_SYS_ID)
            .component_id(DEFAULT_TCP_SERVER_COMP_ID)
            .dialect(minimal::dialect())
            .version(V2)
            .connection(TcpServerConf::new(make_addr(port)).unwrap())
            .build(),
    )
    .unwrap()
}

pub fn make_tcp_client_node(
    port: Port,
    component_id: u8,
) -> Node<Identified, HasDialect<minimal::Minimal>, V2> {
    Node::try_from(
        NodeConf::builder()
            .system_id(DEFAULT_TCP_CLIENT_SYS_ID)
            .component_id(component_id)
            .dialect(minimal::dialect())
            .version(V2)
            .connection(TcpClientConf::new(make_addr(port)).unwrap())
            .build(),
    )
    .unwrap()
}

pub fn make_udp_server_node(port: Port) -> Node<Identified, HasDialect<minimal::Minimal>, V2> {
    Node::try_from(
        NodeConf::builder()
            .system_id(DEFAULT_UDP_SERVER_SYS_ID)
            .component_id(DEFAULT_UDP_SERVER_COMP_ID)
            .dialect(minimal::dialect())
            .version(V2)
            .connection(UdpServerConf::new(make_addr(port)).unwrap())
            .build(),
    )
    .unwrap()
}

pub fn make_udp_client_node(
    port: Port,
    component_id: u8,
) -> Node<Identified, HasDialect<minimal::Minimal>, V2> {
    Node::try_from(
        NodeConf::builder()
            .system_id(DEFAULT_UDP_CLIENT_SYS_ID)
            .component_id(component_id)
            .dialect(minimal::dialect())
            .version(V2)
            .connection(UdpClientConf::new(make_addr(port)).unwrap())
            .build(),
    )
    .unwrap()
}
