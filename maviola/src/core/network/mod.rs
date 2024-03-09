//! # MAVLink network
//!
//! A network is a collection of nodes with different underlying transports.
//!
//! Each message received by one node will be broadcast to other nodes. More specifically, the
//! broadcast operates on the level of channels. That means, that if, for example, a server node
//! receives a message from one of its clients, then this message will be forwarded to all other
//! clients of this server and all other nodes.

mod base;
pub(crate) mod types;

pub use base::Network;
