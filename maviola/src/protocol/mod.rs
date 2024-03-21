//! # MAVLink protocol
//!
//! This module contains MAVLink protocol abstraction. Most of them (such as MAVLink [`Frame`]) are
//! re-exported from [`mavio`](https://crates.io/crates/mavio). A few additional abstractions are
//! related to a high-level nature of Maviola library. All exports from Mavio are marked with
//! <sup>[`mavio`](https://crates.io/crates/mavio)</sup>.

pub mod consts;
#[cfg(feature = "unsafe")]
mod custom;
mod dialects;
mod peer;
mod processor;
mod signature;

pub use dialects::KnownDialects;
pub use peer::Peer;
pub use processor::FrameProcessor;
pub use signature::{
    FrameSigner, FrameSignerBuilder, IntoFrameSigner, SignStrategy, UniqueMavTimestamp,
};

#[cfg(feature = "unsafe")]
pub use custom::{CustomFrameProcessors, ProcessFrame, ProcessFrameCase};
#[cfg(not(feature = "unsafe"))]
#[derive(Clone, Debug, Default)]
pub(crate) struct CustomFrameProcessors;

/// <sup>[`mavio`](https://crates.io/crates/mavio)</sup>
#[doc(inline)]
pub use mavio::protocol::*;

/// <sup>[`mavio`](https://crates.io/crates/mavio)</sup>
#[doc(inline)]
pub use mavio::utils::MavSha256;
