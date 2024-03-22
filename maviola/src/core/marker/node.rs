//! Markers for MAVLink [`Node`](crate::io::Node).

use std::fmt::Debug;

use crate::core::utils::Sealed;
use crate::protocol::{ComponentId, Endpoint, MaybeVersioned, SystemId, Unset};

/// <sup>🔒</sup>
/// All kinds of nodes are falling under this trait.
///
/// 🔒 This trait is sealed 🔒
///
/// Variants:
///
/// * [`Proxy`]
/// * [`Edge`]
pub trait NodeKind: Clone + Debug + Sync + Send + Sealed {}

/// Variant of a node that proxies existing messages.
///
/// This node can't produce messages and can be used only as a proxy with an optional message
/// signing capability.
#[derive(Clone, Copy, Debug)]
pub struct Proxy;
impl Sealed for Proxy {}
impl NodeKind for Proxy {}

/// Variant of a node with `system_id` and `component_id` being defined.
///
/// This node can produce messages.
#[derive(Clone, Debug)]
pub struct Edge<V: MaybeVersioned> {
    pub(crate) endpoint: Endpoint<V>,
}
impl<V: MaybeVersioned> Sealed for Edge<V> {}
impl<V: MaybeVersioned> NodeKind for Edge<V> {}

/// <sup>🔒</sup>
/// Variant of a node configuration which may or may not have a connection config.
///
/// 🔒 This trait is sealed 🔒
pub trait MaybeConnConf: Debug + Send + Sealed {}

impl MaybeConnConf for Unset {}

/// <sup>🔒</sup>
/// Variant of a node configuration which has a connection config.
///
/// 🔒 This trait is sealed 🔒
pub trait HasConnConf: MaybeConnConf {
    /// Returns `true` if it makes sense to restart the node after connection failure.
    ///
    /// A blanket implementation always returns `false`.
    fn is_repairable(&self) -> bool {
        false
    }
}

/// <sup>🔒</sup>
/// Marker trait for an entity with or without MAVLink system `ID`.
///
/// 🔒 This trait is sealed 🔒
pub trait MaybeSystemId: Clone + Copy + Debug + Sync + Send + Sealed {}

impl MaybeSystemId for Unset {}

/// Marker for an entity with a defined MAVLink system `ID`.
#[derive(Copy, Clone, Debug)]
pub struct HasSystemId(pub SystemId);
impl Sealed for HasSystemId {}
impl MaybeSystemId for HasSystemId {}

/// <sup>🔒</sup>
/// Marker trait for an entity with or without MAVLink component `ID`.
///
/// 🔒 This trait is sealed 🔒
pub trait MaybeComponentId: Clone + Debug + Sync + Send + Sealed {}

impl MaybeComponentId for Unset {}

/// Marker for an entity with a defined MAVLink component `ID`.
#[derive(Copy, Clone, Debug)]
pub struct HasComponentId(pub ComponentId);
impl Sealed for HasComponentId {}
impl MaybeComponentId for HasComponentId {}
