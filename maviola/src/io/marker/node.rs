//! Markers for MAVLink [`Node`](super::node::Node).

use crate::utils::Sealed;
use mavio::protocol::{ComponentId, SystemId};

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
    /// MAVLink system `ID`
    pub system_id: SystemId,
    /// MAVLink component `ID`
    pub component_id: ComponentId,
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
pub trait HasConnConf: MaybeConnConf {}
