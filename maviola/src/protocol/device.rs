use std::hash::Hash;
use std::sync::Arc;

use crate::core::marker::{Edge, NodeKind};
use crate::core::node::NodeApi;
use crate::core::utils::UniqueId;
use crate::error::Result;

use crate::prelude::*;
use crate::protocol::*;

/// MAVLink device `ID`.
///
/// This is a wrapper around [`MavLinkId`] that ensures uniqueness of the identifier during the
/// program lifetime. It is possible to have several devices with the same [`MavLinkId`] but
/// different [`DeviceId`]. For example, since [`DeviceId`] implements [`Hash`] you may have
/// a configuration with several endpoints of different processing strategies for the same
/// combination of system / component `ID`.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct DeviceId {
    mavlink_id: MavLinkId,
    unique_id: UniqueId,
}

/// MAVLink device with frame processing configuration derived from a node and defined [`MavLinkId`].
///
/// There are only two ways to create a device. First, directly from the edge node using [`From`].
/// And second, by providing a reference to existing node to [`Device::new`].
///
/// We intentionally bound all devices to nodes since otherwise it would be possible to create a
/// device with "orphan" frame processing settings. Which means you may end up with a device that
/// produces frames no node can actually send.
#[derive(Clone, Debug)]
pub struct Device<V: MaybeVersioned> {
    id: DeviceId,
    processor: Arc<FrameProcessor>,
    endpoint: Endpoint<V>,
}

impl DeviceId {
    /// Creates a new [`DeviceId`] from the provided [`MavLinkId`].
    pub fn new(mavlink_id: MavLinkId) -> Self {
        Self {
            mavlink_id,
            unique_id: UniqueId::new(),
        }
    }

    /// Returns [`MavLinkId`].
    #[inline(always)]
    pub fn mavlink_id(&self) -> MavLinkId {
        self.mavlink_id
    }

    /// Returns MAVLink [`SystemId`].
    #[inline(always)]
    pub fn system_id(&self) -> SystemId {
        self.mavlink_id.system
    }

    /// Returns MAVLink [`ComponentId`].
    #[inline(always)]
    pub fn component_id(&self) -> ComponentId {
        self.mavlink_id.component
    }
}

impl Device<Versionless> {
    /// Creates a new MAVLink device from the provided [`MavLinkId`] and [`Node`].
    ///
    /// This will apply frame processing configuration from the node to the device.
    pub fn new<K: NodeKind, V: MaybeVersioned, A: NodeApi<V>>(
        mavlink_id: MavLinkId,
        node: &Node<K, V, A>,
    ) -> Device<V> {
        Device {
            id: DeviceId::new(mavlink_id),
            processor: node.processor.clone(),
            endpoint: Endpoint::new(mavlink_id),
        }
    }
}

impl<V: MaybeVersioned> Device<V> {
    /// Returns device `ID`.
    #[inline(always)]
    pub fn id(&self) -> DeviceId {
        self.id
    }

    /// Returns a reference to the internal [`Endpoint`].
    #[inline(always)]
    pub fn endpoint(&self) -> &Endpoint<V> {
        &self.endpoint
    }
}

impl<V: Versioned> Device<V> {
    /// Creates a next frame from MAVLink message.
    ///
    /// Ensures that frame has a correct [`sequence`], [`system_id`], and [`component_id`]. Signs
    /// frame and sets [`compat_flags`] / [`incompat_flags`] if internal frame processing
    /// system is defined to do so.
    ///
    /// [`sequence`]: Frame::sequence
    /// [`system_id`]: Frame::system_id
    /// [`component_id`]: Frame::component_id
    /// [`incompat_flags`]: Frame::incompat_flags
    /// [`compat_flags`]: Frame::compat_flags
    pub fn next_frame(&self, message: &dyn Message) -> Result<Frame<V>> {
        let mut frame = self.endpoint.next_frame(message)?;
        self.processor.process_new(&mut frame);
        Ok(frame)
    }
}

impl Device<Versionless> {
    /// Creates a next versionless frame from MAVLink message.
    ///
    /// Requires protocol version to be specified via [turbofish](https://turbo.fish/about) syntax.
    ///
    /// Ensures that frame has a correct [`sequence`], [`system_id`], and [`component_id`]. Signs
    /// frame and sets [`compat_flags`] / [`incompat_flags`] if internal frame processing system
    /// is defined to do so.
    ///
    /// [`sequence`]: Frame::sequence
    /// [`system_id`]: Frame::system_id
    /// [`component_id`]: Frame::component_id
    /// [`incompat_flags`]: Frame::incompat_flags
    /// [`compat_flags`]: Frame::compat_flags
    pub fn next_frame_versioned<V: Versioned>(
        &self,
        message: &dyn Message,
    ) -> Result<Frame<Versionless>> {
        let mut frame = self.endpoint.next_frame::<V>(message)?;
        self.processor.process_new(&mut frame);
        Ok(frame)
    }
}

impl<V: MaybeVersioned, A: NodeApi<V>> From<&Node<Edge<V>, V, A>> for Device<V> {
    fn from(value: &Node<Edge<V>, V, A>) -> Self {
        Self {
            id: value.kind.device_id,
            processor: value.processor.clone(),
            endpoint: value.kind.endpoint.clone(),
        }
    }
}
