//! # Low-level I/O abstractions
//!
//! This module re-exports [`Sender`] / [`Receiver`] for synchronous I/O and [`AsyncSender`] /
//! [`AsyncReceiver`] for the synchronous one from [Mavio](https://crates.io/crates/mavio).
//! Technically speaking, the entire library is an extension for these low-level primitives. It is
//! possible to combine high-level and low-level tools since both are using the same MAVLink
//! abstractions defined in [`protocol`](crate::protocol) (with a handful of extras).
//!
//! All low-level MAVLink abstractions are available in [`crate::core`].

#[doc(inline)]
#[cfg(feature = "sync")]
/// <sup>`sync`</sup>
/// <sup>| [`mavio`](https://crates.io/crates/mavio)</sup>
pub use crate::core::io::{Receiver, Sender};

#[doc(inline)]
#[cfg(feature = "async")]
/// <sup>`async`</sup>
/// <sup>| [`mavio`](https://crates.io/crates/mavio)</sup>
pub use crate::core::io::{AsyncReceiver, AsyncSender};
