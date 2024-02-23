//! Common utils.

pub mod closable;
mod flipper;
pub(crate) mod net;
pub(crate) mod sealed;
#[cfg(test)]
#[allow(dead_code)]
pub(crate) mod test;
mod unique_id;

#[doc(inline)]
pub use closable::{Closable, Closer, SharedCloser};
#[doc(inline)]
pub use flipper::{Flag, Flipper, Guarded, Switch};

pub(crate) use sealed::Sealed;
pub(crate) use unique_id::UniqueId;
