//! # Maviola synchronous I/O

/// <sup>[`mavio`](https://docs.rs/mavio/0.2.0-rc2/mavio/)</sup>
pub use mavio::{Receiver, Sender};

pub(crate) mod connection;
mod event;
pub mod mpmc;
mod mpsc_rw;
mod node;
pub(crate) mod response;
mod sock;
mod tcp;
mod udp;

pub use connection::Connection;
pub use event::Event;
pub use node::Node;
pub use response::Response;
pub use tcp::client::TcpClientConf;
pub use tcp::server::TcpServerConf;
pub use udp::client::UdpClientConf;
pub use udp::server::UdpServerConf;

#[cfg(unix)]
/// <sup>`unix`</sup>
pub use sock::client::SockClientConf;
#[cfg(unix)]
/// <sup>`unix`</sup>
pub use sock::server::SockServerConf;
