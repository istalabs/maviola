//! # MAVLink protocol
//!
//! This module contains MAVLink protocol abstraction. Most of them (such as MAVLink [`Frame`]) are
//! re-exported from [`Mavio`](https://crates.io/crates/mavio). A few additional abstractions are
//! related to a high-level nature of Maviola library. All exports from Mavio are marked with
//! <sup>[`mavio`](https://crates.io/crates/mavio)</sup>.
//!
//! If `derive` feature is enabled, we also import derive macros from
//! [`MAVSpec`](https://crates.io/crates/mavspec). These macros are marked with
//! <sup>[`mavspec`](https://crates.io/crates/mavspec)</sup>.

pub mod consts;
#[cfg(feature = "unsafe")]
mod custom;
mod device;
mod dialects;
mod peer;
mod processor;
mod signature;

pub use device::{Device, DeviceId};
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

/// <sup>[`mavspec`](https://crates.io/crates/mavspec)</sup>
///
/// # Derive macros from [MAVSpec](https://crates.io/crates/mavspec)
///
/// These macros allow to derive [`Dialect`], [`Message`] and MAVLink enums.
///
/// [`Dialect`]: crate::protocol::Dialect
/// [`Message`]: crate::protocol::Message
///
/// ---
#[cfg(feature = "derive")]
pub mod derive {
    /// <sup>[`mavspec`](https://crates.io/crates/mavspec)</sup>
    #[doc(inline)]
    pub use mavspec::rust::derive::{Dialect, Enum, Message};
}
