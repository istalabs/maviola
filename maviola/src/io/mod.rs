//! # Maviola I/O
//!
//! This module contains both synchronous and asynchronous API as well as several abstractions and
//! utilities common to all API modes.
//!
//! ## API modes
//!
//! Synchronous API lives in [`sync`] module, a handful of the most important entities re-exported
//! on the higher level are marked with <sup>[`sync`]</sup>.
//!
//! Asynchronous API is based on [Tokio](https://crates.io/crates/tokio) runtime and lives in
//! [`asnc`] module, a handful of the most important entities re-exported
//! on the higher level are marked with <sup>[`async`](asnc)</sup>.
//!
//! ## Low-level I/O
//!
//! Low-level I/O primitives are available in [`core`] module. Most of these abstractions are
//! re-exported from [Mavio](https://crates.io/crates/mavio), a low-level MAVLink library which
//! serves as a basis for Maviola.

#[cfg(feature = "sync")]
pub mod asnc;
mod broadcast;
mod connection_info;
pub mod core;
pub mod marker;
mod node_builder;
mod node_conf;
#[cfg(feature = "sync")]
pub mod sync;
mod utils;

pub use broadcast::OutgoingFrame;
pub use connection_info::{ChannelInfo, ConnectionInfo};
pub use node_builder::NodeBuilder;
pub use node_conf::NodeConf;

#[doc(inline)]
#[cfg(feature = "sync")]
/// <sup>[`sync`]</sup>
pub use sync::{Callback, Event, Node};
#[doc(inline)]
#[cfg(feature = "sync")]
/// <sup>[`sync`]</sup>
pub use sync::{FileReader, FileWriter, TcpClient, TcpServer, UdpClient, UdpServer};
#[doc(inline)]
#[cfg(feature = "sync")]
/// <sup>[`sync`] |</sup>
pub use sync::{SockClient, SockServer};

#[doc(inline)]
#[cfg(feature = "async")]
/// <sup>[`async`](asnc)</sup>
pub use asnc::{AsyncConnection, AsyncResponse};
