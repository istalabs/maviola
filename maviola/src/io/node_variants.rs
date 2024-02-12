//! Markers for MAVLink [`Node`](super::node::Node).

/// Marker for [`Node`](super::node::Node) with or without
/// [`system_id`](super::node::Node::system_id) and [`component_id`](super::node::Node::component_id).
///
/// Variants:
///
/// * [`NotIdentified`]
/// * [`Identified`]
pub trait IsIdentified: Clone {}

/// Variant of [`Node`](super::node::Node) without [`system_id`](super::node::Node::system_id)
/// and [`component_id`](super::node::Node::component_id).
///
/// This node can't produce messages and can be used only as a proxy.
#[derive(Clone)]
pub struct NotIdentified;
impl IsIdentified for NotIdentified {}

/// Variant of [`Node`](super::node::Node) with [`system_id`](super::node::Node::system_id)
/// and [`component_id`](super::node::Node::component_id) being defined.
///
/// This node can produce messages.
#[derive(Clone)]
pub struct Identified {
    pub(crate) system_id: u8,
    pub(crate) component_id: u8,
}
impl IsIdentified for Identified {}
