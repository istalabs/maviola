//! # MAVLink protocol abstractions
//!
//! This module re-export entities form
//! [`mavio::protocol`](https://docs.rs/mavio/0.2.0-rc2/mavio/protocol/) to provide full access to
//! MAVLink abstractions.

pub mod consts;
mod marker;
mod peer;
mod signature;

pub use peer::Peer;
pub use signature::{SignConf, SignConfBuilder, SignStrategy};

pub use marker::{Dialectless, HasDialect, MaybeDialect};

/// <sup>[`mavio`](https://docs.rs/mavio/0.2.0-rc2/mavio/protocol/)</sup>
#[doc(inline)]
pub use mavio::protocol::*;

pub(crate) use peer::PeerId;
