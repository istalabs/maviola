//! MAVLink protocol entities.

pub mod frame;
pub mod signature;
pub mod variants;

#[doc(inline)]
pub use frame::{CoreFrame, Frame};
#[doc(inline)]
pub use signature::{SignConf, SignStrategy};
