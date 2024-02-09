//! Variants for [`Node`](super::node::Node).

use mavio::protocol::{DialectImpl, DialectMessage, MavLinkVersion};

/// Marker for [`Node`](super::node::Node) with or without [`dialect`](super::node::Node::dialect).
///
/// Variants:
///
/// * [`NoDialect`]
/// * [`Dialect`]
pub trait HasDialect: Clone {}

/// Variant of [`Node`](super::node::Node) without a [`dialect`](super::node::Node::dialect).
///
/// Message decoding is not possible, only raw MAVLink frames can be communicated.
#[derive(Clone)]
pub struct NoDialect();
impl HasDialect for NoDialect {}

/// Variant of [`Node`](super::node::Node) with a [`dialect`](super::node::Node::dialect) being
/// specified.
///
/// Message encoding/decoding is available.
pub struct Dialect<M: DialectMessage + 'static>(pub(crate) &'static dyn DialectImpl<Message = M>);
impl<M: DialectMessage + 'static> Clone for Dialect<M> {
    fn clone(&self) -> Self {
        Dialect(self.0)
    }
}
impl<M: DialectMessage + 'static> HasDialect for Dialect<M> {}

/// Marker for [`Node`](super::node::Node) with or without
/// [`system_id`](super::node::Node::system_id) and [`component_id`](super::node::Node::component_id).
///
/// Variants:
///
/// * [`NotIdentified`]
/// * [`Identified`]
pub trait HasIdentifier: Clone {}

/// Variant of [`Node`](super::node::Node) without [`system_id`](super::node::Node::system_id)
/// and [`component_id`](super::node::Node::component_id).
///
/// This node can't produce messages and can be used only as a proxy.
#[derive(Clone)]
pub struct NotIdentified();
impl HasIdentifier for NotIdentified {}

/// Variant of [`Node`](super::node::Node) with [`system_id`](super::node::Node::system_id)
/// and [`component_id`](super::node::Node::component_id) being defined.
///
/// This node can produce messages.
#[derive(Clone)]
pub struct Identified {
    pub(crate) system_id: u8,
    pub(crate) component_id: u8,
}
impl HasIdentifier for Identified {}

/// Marker for [`Node`](super::node::Node) with or without [`version`](super::node::Node::version).
///
/// Variants:
///
/// * [`NotVersioned`]
/// * [`Versioned`]
pub trait HasVersion: Clone {}

/// Variant of [`Node`](super::node::Node) without predefined MAVLink [`version`](super::node::Node::version).
///
/// Nodes of such type can send and receive messages of any MAVLink version.
#[derive(Clone)]
pub struct NotVersioned();
impl HasVersion for NotVersioned {}

/// Variant of [`Node`](super::node::Node) with a defined MAVLink [`version`](super::node::Node::version).
///
/// Nodes of such type can send only messages within a specific MAVLink protocol version.
#[derive(Clone)]
pub struct Versioned(pub(crate) MavLinkVersion);

impl HasVersion for Versioned {}
