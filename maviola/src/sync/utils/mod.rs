//! # <sup>`⍚`</sup> Synchronous utils
//!
//! > ⚠ This part of the API is exposed for those who want to implement [`ConnectionBuilder`] and
//! > create custom connections. It is still considered experimental and available only under
//! > `unstable` feature being enabled.

#[cfg(doc)]
use crate::sync::io::ConnectionBuilder;

mod busy_rw;
mod handlers;
pub mod mpmc;
mod mpsc_rw;

pub use busy_rw::{BusyReader, BusyWriter};
pub use mpsc_rw::{MpscReader, MpscWriter};

pub(crate) use handlers::handle_listener_stop;
