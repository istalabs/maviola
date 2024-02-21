//! # I/O generic markers
//!
//! These markers are used to distinguish different versions of generic entities.

mod node;

pub use node::{HasConnConf, Identified, MaybeConnConf, MaybeIdentified, NoConnConf, Unidentified};
