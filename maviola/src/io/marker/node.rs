//! Markers for MAVLink [`Node`](crate::io::Node).

use mavio::protocol::{ComponentId, SystemId};

use crate::utils::Sealed;

/// <sup>🔒</sup>
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

/// <sup>🔒</sup>
/// Variant of a node configuration which may or may not have a connection config.
///
/// ⚠ This trait is sealed ⚠
pub trait MaybeConnConf: Sealed {}

/// Variant of a node configuration without a connection config.
pub struct NoConnConf;
impl Sealed for NoConnConf {}
impl MaybeConnConf for NoConnConf {}

/// <sup>🔒</sup>
/// Variant of a node configuration which has a connection config.
///
/// ⚠ This trait is sealed ⚠
pub trait HasConnConf: MaybeConnConf {}

/// <sup>🔒</sup>
/// Marker trait for an entity with or without MAVLink system `ID`.
///
/// ⚠ This trait is sealed ⚠
pub trait MaybeSystemId: Sealed {}

/// Marker for an entity without MAVLink system `ID`.
pub struct NoSystemId;
impl Sealed for NoSystemId {}
impl MaybeSystemId for NoSystemId {}

/// Marker for an entity with a defined MAVLink system `ID`.
pub struct HasSystemId(pub SystemId);
impl Sealed for HasSystemId {}
impl MaybeSystemId for HasSystemId {}

/// <sup>🔒</sup>
/// Marker trait for an entity with or without MAVLink component `ID`.
///
/// ⚠ This trait is sealed ⚠
pub trait MaybeComponentId: Sealed {}

/// Marker for an entity without MAVLink component `ID`.
pub struct NoComponentId;
impl Sealed for NoComponentId {}
impl MaybeComponentId for NoComponentId {}

/// Marker for an entity with a defined MAVLink component `ID`.
pub struct HasComponentId(pub ComponentId);
impl Sealed for HasComponentId {}
impl MaybeComponentId for HasComponentId {}
