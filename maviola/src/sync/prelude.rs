//! # Common imports for synchronous API
//!
//! Imports essential abstractions, that are required to work with synchronous I/O.
//!
//! ⚠ Incompatible with [`asnc::prelude`](crate::asnc::prelude)!

pub use crate::sync::node::{Callback, EdgeNode, Event, FrameSender, ProxyNode, SyncApi};

pub(crate) use crate::sync::utils::mpmc;
