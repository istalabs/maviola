//! # Synchronous I/O utils

mod busy_rw;
mod handlers;
pub mod mpmc;
mod mpsc_rw;

pub use busy_rw::{BusyReader, BusyWriter};
pub use mpsc_rw::{MpscReader, MpscWriter};

pub(crate) use handlers::handle_listener_stop;
