//! Common utils.

pub mod closable;
pub(crate) mod sealed;
#[cfg(test)]
#[allow(dead_code)]
pub(crate) mod test;
mod unique_id;

#[doc(inline)]
pub use closable::{Closable, Closer, SharedCloser};

pub(crate) use sealed::Sealed;
pub(crate) use unique_id::UniqueId;
