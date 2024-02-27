//! # Maviola prelude
//!
//! This module contains basic imports for Maviola.

pub use crate::core::error::{Error, Result};
pub use crate::core::error::{NodeError, SyncError};
pub use crate::core::node::Node;
pub use crate::dialects::Minimal;
pub use crate::protocol::{
    Dialect, Endpoint, Frame, MavLinkId, MavLinkVersion, MaybeVersioned, Message, Versioned,
    Versionless, V1, V2,
};

#[cfg(feature = "sync")]
pub(crate) use crate::sync::utils::mpmc;

#[cfg(feature = "async")]
pub(crate) use tokio::sync::broadcast;
