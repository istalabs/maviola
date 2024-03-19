//! # Core I/O abstractions
//!
//! This module contains both synchronous and asynchronous API as well as several abstractions and
//! utilities common to all API modes.
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
//! ## API modes
//!
//! Synchronous API lives in [`sync`](crate::sync) module, and marked with
//! <sup>[`sync`](crate::sync)</sup>.
//!
//! Asynchronous API is based on [Tokio](https://crates.io/crates/tokio) runtime, lives in
//! [`asnc`](crate::asnc) module, a and marked with <sup>[`async`](crate::asnc)</sup>.
//!
//! ## Low-level I/O
//!
//! Low-level I/O primitives are re-exported from [Mavio](https://crates.io/crates/mavio), a
//! low-level MAVLink library which serves as a basis for Maviola.

mod connection_conf;
mod connection_info;
mod core;
mod retry;
mod routing;
mod transport;

pub use transport::{FileReader, FileWriter, TcpClient, TcpServer, UdpClient, UdpServer};
#[cfg(unix)]
pub use transport::{SockClient, SockServer};

pub use connection_conf::ConnectionConf;
pub use connection_info::{ChannelDetails, ChannelInfo, ConnectionDetails, ConnectionInfo};
pub use retry::Retry;
pub use routing::{BroadcastScope, ChannelId, ConnectionId};

#[cfg(feature = "unstable")]
pub use routing::{IncomingFrame, OutgoingFrame};
#[cfg(not(feature = "unstable"))]
pub(crate) use routing::{IncomingFrame, OutgoingFrame};

#[cfg(feature = "sync")]
/// <sup>[`mavio`](https://crates.io/crates/mavio) | `sync`</sup>
#[doc(inline)]
pub use core::{Receiver, Sender};

#[cfg(feature = "async")]
/// <sup>[`mavio`](https://crates.io/crates/mavio) | `asnc`</sup>
#[doc(inline)]
pub use core::{AsyncReceiver, AsyncSender};
