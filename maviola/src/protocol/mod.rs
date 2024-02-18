//! MAVLink protocol entities.

mod peer;
mod signature;

pub use peer::Peer;
pub use signature::{SignConf, SignConfBuilder, SignStrategy};

pub(crate) use peer::PeerId;
