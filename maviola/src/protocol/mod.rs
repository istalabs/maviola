//! # MAVLink protocol
//!
//! This module contains MAVLink protocol abstraction. Most of them (such as MAVLink [`Frame`]) are
//! re-exported from [`Mavio`](https://crates.io/crates/mavio). A few additional abstractions are
//! related to a high-level nature of Maviola library. All exports from Mavio are marked with
//! <sup>[`mavio`](https://crates.io/crates/mavio)</sup>.
//!
//! MAVSpec entities are also re-exported and marked with
//! <sup>[`mavspec`](https://crates.io/crates/mavspec)</sup>.
//!
//! ## Dialects
//!
//! MAVLink dialects available at [`dialects`] module and require corresponding `dlct-*` feature
//! flag to be enabled. However, [`minimal`](dialects::minimal) dialect is always available.
//!
//! ## Derive Macros
//!
//! If `derive` feature is enabled, we also import [`derive`](mod@derive) macros from
//! [`MAVSpec`](https://crates.io/crates/mavspec). If you want to use these macros, make sure that
//! you've imported [`mavspec`] either from [`maviola::protocol::mavspec`](mavspec) or through
//! [`maviola::prelude`](crate::prelude).
//!
//! You should enable `derive` feature flag to access this functionality.
//!
//! ## MAVLink Microservices
//!
//! MAVLink [microservices](https://mavlink.io/en/services/) are generated as a part of the
//! [default dialect](default_dialect). You can access them via [`default_dialect::microservices`].
//! Make sure that you've enabled corresponding `msrv-*` feature flag.
//!
//! Additional microservice utils are available as [`microservices`] and require `msrv-utils-*`
//! feature flags to be enabled.
//!
//! ## Message Definitions
//!
//! MAVLink [message definitions](https://gitlab.com/mavka/spec/protocols/mavlink/message-definitions-v1.0)
//! can be accessed via [`definitions`] and require `definitions` feature flag to be enabled.

pub mod consts;
#[cfg(feature = "unsafe")]
mod custom;
mod device;
mod dialects_utils;
mod mavspec_imports;
mod peer;
mod processor;
mod signature;

pub use device::{Device, DeviceId};
pub use dialects_utils::KnownDialects;
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

pub use mavspec_imports::*;
