//! Common generic markers.
//!
//! These markers are used to distinguish different versions of generic entities.

mod dialect;

pub use dialect::{Dialectless, HasDialect, MaybeDialect};
