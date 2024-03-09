//! # Maviola asynchronous I/O
//!
//! Synchronous API is built around MAVlink [`Node`] with [`AsyncApi`]. Upon construction, each node
//! operates on a particular connection. The latter is owned by a node and defines underlying
//! transport (e.g. TCP, UDP, Unix socket). Each connection spawns one or several channels. For
//! example, TCP server creates a channel per each incoming connection. Abstractions related to
//! channels and connections are defined in the [`io`] module.
//!
//! ## Usage
//!
//! To use asynchronous API, you have to configure node to use asynchronous connection using
//! [`NodeBuilder::async_connection`](crate::core::node::NodeBuilder::async_connection).
//! For example, the following snippet will create an asynchronous TCP client:
//!
//! ```rust,no_run
//! # #[tokio::main] async fn main() {
//! use maviola::prelude::*;
//!
//! let addr = "127.0.0.1:5600";
//!
//! // Create a TCP client node
//! let node = Node::builder()
//!         /* define other node parameters */
//! #       .version::<V2>()
//! #       .system_id(1)
//! #       .component_id(1)
//!         .async_connection(
//!             TcpClient::new(addr)    // Configure TCP client connection
//!                 .unwrap()
//!         ).build().await.unwrap();
//! # }
//! ```
//!
//! ## Transport
//!
//! The following transports are currently available:
//!
//! * TCP: [`TcpServer`] / [`TcpClient`]
//! * UDP: [`UdpServer`] / [`UdpClient`]
//! * File: [`FileWriter`] / [`FileReader`]
//! * Unix socket: [`SockServer`] / [`SockClient`] (only on Unix-like systems such as Linux or OS X)
//!
//! Connection-level information about each transport is available as a variant of
//! [`ConnectionInfo`](crate::core::io::ConnectionInfo). Channel information is provided by
//! [`ChannelInfo`](crate::core::io::ChannelInfo).
//!
//! ## Events
//!
//! The suggested approach for handling connection with several MAVLink devices is subscribing for
//! [`Node::events`]. This method provides an iterator over all node events. Incoming frames are
//! emitted as [`Event::Frame`]. Such events contain a [`Frame`] / [`Callback`] pair. The latter can
//! be used to respond to a channel from which frame was received or broadcast it to all channels
//! (or, alternatively, to all channels except the one which delivered the original frame).
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
//! [`ConnectionBuilder`](io::ConnectionBuilder) trait. For Custom connections there are reserved
//! [`ConnectionInfo::Custom`](crate::core::io::ConnectionInfo::Custom) and
//! [`ChannelInfo::Custom`](crate::core::io::ChannelInfo::Custom) variants. Check for other relevant
//! abstractions in [`io`] module.
//!
//! ## Low-level I/O
//!
//! Low-level I/O primitives are available in [`crate::core::io`]. Most of these abstractions are
//! re-exported from [Mavio](https://crates.io/crates/mavio), a low-level MAVLink library which
//! serves as a basis for Maviola.

#[cfg(doc)]
use crate::asnc::prelude::*;
#[cfg(doc)]
use crate::prelude::*;

mod consts;
pub mod io;
pub mod marker;
pub mod node;
pub mod prelude;

mod network;
#[cfg(not(feature = "unstable"))]
pub(crate) mod utils;
#[cfg(feature = "unstable")]
pub mod utils;
