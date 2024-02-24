//! # Core abstractions.

pub mod consts;
pub mod error;
pub mod io;
pub mod marker;
pub(crate) mod node;
pub mod utils;

pub use node::Node;
pub use node::NodeBuilder;
pub use node::NodeConf;
