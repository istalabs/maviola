//! Markers for MAVLink [`Node`](super::node::Node).

#[cfg(feature = "sync")]
use mavio::protocol::MaybeVersioned;

#[cfg(feature = "sync")]
use crate::io::sync::connection::ConnectionConf;

use crate::utils::Sealed;

#[cfg(feature = "sync")]
pub use sync_conn_conf::*;

/// Marker for a node with or without `system_id` and `component_id`.
///
/// ⚠ This trait is sealed ⚠
///
/// Variants:
///
/// * [`Unidentified`]
/// * [`Identified`]
pub trait MaybeIdentified: Clone + Sealed {}

/// Variant of a node without `system_id` and `component_id`.
///
/// This node can't produce messages and can be used only as a proxy.
#[derive(Clone)]
pub struct Unidentified;
impl Sealed for Unidentified {}
impl MaybeIdentified for Unidentified {}

/// Variant of a node with `system_id` and `component_id` being defined.
///
/// This node can produce messages.
#[derive(Clone)]
pub struct Identified {
    pub(crate) system_id: u8,
    pub(crate) component_id: u8,
}
impl Sealed for Identified {}
impl MaybeIdentified for Identified {}

/// Variant of a node configuration which may or may not have a connection config.
///
/// ⚠ This trait is sealed ⚠
pub trait MaybeConnConf: Sealed {}

/// Variant of a node configuration without a connection config.
pub struct NoConnConf;
impl Sealed for NoConnConf {}
impl MaybeConnConf for NoConnConf {}

/// Variant of a node configuration which has a connection config.
pub trait ConnConf: MaybeConnConf {}

#[cfg(feature = "sync")]
mod sync_conn_conf {
    use super::*;

    /// Variant of a node configuration which has a synchronous connection config.
    pub struct SyncConnConf<V: MaybeVersioned>(pub(crate) Box<dyn ConnectionConf<V>>);
    impl<V: MaybeVersioned> Sealed for SyncConnConf<V> {}
    impl<V: MaybeVersioned> ConnConf for SyncConnConf<V> {}
    impl<V: MaybeVersioned> MaybeConnConf for SyncConnConf<V> {}
}
