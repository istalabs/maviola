//! Common utils.

mod sealed;
pub mod sync;
mod unique_id;

pub(crate) use sealed::Sealed;
pub use unique_id::UniqueId;
