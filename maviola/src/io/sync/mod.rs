//! # Maviola synchronous I/O

/// <sup>[`mavio`](https://docs.rs/mavio/0.2.0-rc2/mavio/)</sup>
pub use mavio::{Receiver, Sender};

mod callback;
pub(crate) mod connection;
mod consts;
mod event;
mod file;
pub mod marker;
pub mod mpmc;
mod mpsc_rw;
mod node;
mod sock;
mod tcp;
mod udp;
pub mod utils;

pub use callback::Callback;
pub use connection::Connection;
pub use event::Event;
pub use file::reader::FileReader;
pub use file::writer::FileWriter;
pub use node::Node;
pub use tcp::client::TcpClient;
pub use tcp::server::TcpServer;
pub use udp::client::UdpClient;
pub use udp::server::UdpServer;

#[cfg(unix)]
/// <sup>`unix`</sup>
pub use sock::client::SockClient;
#[cfg(unix)]
/// <sup>`unix`</sup>
pub use sock::server::SockServer;
