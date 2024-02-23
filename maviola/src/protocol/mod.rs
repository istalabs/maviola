//! # MAVLink protocol abstractions
//!
//! This module contains MAVLink protocol abstraction. Most of them (such as MAVLink [`Frame`]) are
//! re-exported from [`mavio`](https://crates.io/crates/mavio). A few additional abstractions are
//! related to a high-level nature of Maviola library. All exports from Mavio are marked with
//! <sup>[`mavio`](https://crates.io/crates/mavio)</sup> and also available in
//! [`crate::core::protocol`] ad [`crate::core::consts`] module.

pub mod consts;
mod peer;
mod signature;

pub use peer::Peer;
pub use signature::{SignConf, SignConfBuilder, SignStrategy};

/// <sup>[`mavio`](https://crates.io/crates/mavio)</sup>
#[doc(inline)]
pub use crate::core::protocol::*;

pub(crate) use peer::PeerId;
