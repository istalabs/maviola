//! # Generic markers
//!
//! These markers are used to distinguish different versions of generic entities.

mod node;

pub use node::{
    Edge, HasComponentId, HasConnConf, HasSystemId, MaybeComponentId, MaybeConnConf, MaybeSystemId,
    NodeKind, Proxy,
};

#[doc(inline)]
pub use crate::protocol::Unset;
