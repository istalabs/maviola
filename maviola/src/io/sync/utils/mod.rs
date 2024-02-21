//! # Synchronous I/O utils

mod busy_rw;
mod handlers;

pub use busy_rw::{BusyReader, BusyWriter};
pub(super) use handlers::handle_listener_stop;
