//! # Core I/O abstractions
//!
//! This module contains both synchronous and asynchronous API as well as several abstractions and
//! utilities common to all API modes.
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

mod broadcast;
mod connection_info;
mod core;

pub(crate) use broadcast::BroadcastScope;
pub use broadcast::OutgoingFrame;
pub use connection_info::{ChannelInfo, ConnectionInfo};

#[cfg(feature = "sync")]
/// <sup>[`mavio`](https://crates.io/crates/mavio) | `sync`</sup>
#[doc(inline)]
pub use core::{Receiver, Sender};

#[cfg(feature = "async")]
/// <sup>[`mavio`](https://crates.io/crates/mavio) | `asnc`</sup>
#[doc(inline)]
pub use core::{AsyncReceiver, AsyncSender};
