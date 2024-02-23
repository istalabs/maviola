//! # Generic markers
//!
//! These markers are used to distinguish different versions of generic entities.

mod node;

pub use node::{
    HasComponentId, HasConnConf, HasSystemId, Identified, MaybeComponentId, MaybeConnConf,
    MaybeIdentified, MaybeSystemId, NoComponentId, NoConnConf, NoSystemId, Unidentified,
};
