//! # MAVLink protocol
//!
//! This module contains MAVLink protocol abstraction. Most of them (such as MAVLink [`Frame`]) are
//! re-exported from [`mavio`](https://crates.io/crates/mavio). A few additional abstractions are
//! related to a high-level nature of Maviola library. All exports from Mavio are marked with
//! <sup>[`mavio`](https://crates.io/crates/mavio)</sup>.

pub mod consts;
mod peer;
mod signature;

pub use peer::Peer;
pub use signature::{MessageSigner, MessageSignerBuilder, SignStrategy, UniqueMavTimestamp};

/// <sup>[`mavio`](https://crates.io/crates/mavio)</sup>
#[doc(inline)]
pub use mavio::protocol::*;

/// <sup>[`mavio`](https://crates.io/crates/mavio)</sup>
#[doc(inline)]
pub use mavio::utils::MavSha256;
