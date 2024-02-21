//! # Maviola synchronous I/O
//!
//! Synchronous API is built around MAVlink [`Node`] abstraction. Upon construction, each node
//! operates on a particular connection. The latter is owned by a node and defines underlying
//! transport (e.g. TCP, UDP, Unix socket). Each connection spawns one or several channels.
//! For example, TCP server creates a channel per each incoming connection. Abstractions related to
//! channels and connections are defined in the [`conn`] module.
//!
//! Available transports:
//!
//! * TCP: [`TcpServer`] / [`TcpClient`]
//! * UDP: [`UdpServer`] / [`UdpClient`]
//! * File: [`FileWriter`] / [`FileReader`]
//! * Unix socket: [`SockServer`] / [`SockClient`] (only on Unix-like systems like Linux or OS X)
//!
//! Connection-level information about each transport is available as a variant of
//! [`ConnectionInfo`](super::ConnectionInfo). Channel information is provided by
//! [`ChannelInfo`](super::ChannelInfo).
//!
//! ## Events
//!
//! The suggested approach for handling several MAVLink devices, is to use [`Node::events`]. This
//! method provides an iterator over all node events. Incoming frames are emitted as
//! [`Event::Frame`]. Such events contain a [`Frame`](crate::Frame) / [`Callback`] pair. The
//! latter can be used to respond to a channel from which frame was received or broadcast it to
//! all channels (or, alternatively, to all channels except the one which delivered the original
//! frame).
//!
//! ### Peers
//!
//! Each node handles incoming frame and monitors MAVLink devices represented as
//! [`Peer`](crate::protocol::Peer) objects using MAVLink
//! [heartbeat](https://mavlink.io/en/services/heartbeat.html) protocol. Upon discovery of a peer,
//! an [`Event::NewPeer`] event is emitted. When peers is lost due to missing heartbeats, then
//! [`Event::PeerLost`] is emitted.
//!
//! It is possible to get a list of active peers by [`Node::peers`] or check for peers availability
//! using [`Node::has_peers`].
//!
//! ## Custom connections
//!
//! It is possible to create a custom connection by implementing a
//! [`ConnectionBuilder`](conn::ConnectionBuilder) trait. For Custom connections there are reserved
//! [`ConnectionInfo::Custom`](super::ConnectionInfo::Custom) and
//! [`ChannelInfo::Custom`](super::ChannelInfo::Custom) variants. Check for other relevant
//! abstractions in [`conn`] module.
//!
//! ## Low-level I/O
//!
//! Low-level I/O primitives are available in [`crate::io::core`]. Most of these abstractions are
//! re-exported from [Mavio](https://crates.io/crates/mavio), a low-level MAVLink library which
//! serves as a basis for Maviola.

mod callback;
pub mod conn;
mod consts;
mod event;
mod file;
pub mod marker;
mod node;
mod sock;
mod tcp;
mod udp;
pub mod utils;

pub use callback::Callback;
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
