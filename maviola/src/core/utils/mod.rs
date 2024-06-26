//! Common utils.

pub mod closable;
mod flipper;
mod heartbeat;
pub(crate) mod net;
#[cfg(feature = "sync")]
mod ring;
pub(crate) mod sealed;
#[cfg(test)]
#[allow(dead_code)]
pub(crate) mod test;
mod unique_id;

#[doc(inline)]
pub use closable::{Closable, Closer, SharedCloser};
#[doc(inline)]
pub use flipper::{Flag, Flipper, Guarded, Switch};

#[cfg(feature = "unsafe")]
pub use mavio::utils::TryUpdateFrom;

pub(crate) use heartbeat::make_heartbeat_message;
pub(crate) use sealed::Sealed;
pub(crate) use unique_id::UniqueId;

#[cfg(feature = "sync")]
pub(crate) use ring::RingBuffer;
