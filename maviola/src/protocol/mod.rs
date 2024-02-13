//! MAVLink protocol entities.

pub mod marker;
pub mod signature;

#[doc(inline)]
pub use signature::{SignConf, SignStrategy};
