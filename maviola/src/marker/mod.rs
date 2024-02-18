//! Common generic markers.
//!
//! These markers are used to distinguish different versions of generic entities.

mod dialect;
mod node;

pub use dialect::{Dialectless, HasDialect, MaybeDialect};
#[cfg(feature = "sync")]
pub use node::SyncConnConf;
pub use node::{ConnConf, Identified, IsIdentified, MaybeConnConf, NoConnConf, NotIdentified};
