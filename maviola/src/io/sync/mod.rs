//! # Maviola synchronous I/O

/// <sup>[`mavio`](https://docs.rs/mavio/0.2.0-rc2/mavio/)</sup>
pub use mavio::{Receiver, Sender};

mod callback;
pub(crate) mod connection;
mod consts;
mod event;
pub mod mpmc;
mod mpsc_rw;
mod node;
mod sock;
mod tcp;
mod udp;
mod utils;

pub use callback::Callback;
pub use connection::Connection;
pub use event::Event;
pub use node::Node;
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
