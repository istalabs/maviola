//! # Maviola prelude
//!
//! This module contains basic imports for Maviola.

pub use crate::core::error::{Error, Result};
pub use crate::core::error::{NodeError, SyncError};
pub use crate::dialects::Minimal;
pub use crate::protocol::{Dialect, Frame, MavLinkVersion, Message, V1, V2};

pub(crate) use crate::protocol::{MaybeVersioned, Versioned, Versionless};

#[cfg(feature = "sync")]
pub(crate) use crate::sync::utils::mpmc;

#[cfg(feature = "async")]
pub(crate) use tokio::sync::broadcast;
