//! # Common generic markers
//!
//! These markers are used to distinguish different versions of generic entities.

mod dialect;
mod node;

pub use dialect::{Dialectless, HasDialect, MaybeDialect};
pub use node::{ConnConf, Identified, MaybeConnConf, MaybeIdentified, NoConnConf, Unidentified};

#[cfg(feature = "sync")]
pub use node::SyncConnConf;
