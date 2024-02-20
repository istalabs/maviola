//! # Maviola asynchronous I/O

/// <sup>[`mavio`](https://docs.rs/mavio/0.2.0-rc2/mavio/)</sup>
pub use mavio::{AsyncReceiver, AsyncSender};

pub(crate) mod connection;
mod response;

pub use connection::AsyncConnection;
pub use response::AsyncResponse;
