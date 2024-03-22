use crate::core::io::{BroadcastScope, ConnectionInfo};
use crate::core::utils::Sealed;
use crate::protocol::{FrameProcessor, Unset};

use crate::prelude::*;

/// <sup>⛔</sup>
/// Internal node API.
pub trait NodeApiInternal<V: MaybeVersioned>: Sealed {
    /// <sup>⛔</sup>
    /// Provides information about connection.
    fn info(&self) -> &ConnectionInfo;

    /// <sup>⛔ | 💢</sup>
    /// Routes MAVLink frame without any changes.
    ///
    /// There is nothing particularly unsafe in this method in the sense of unsafe Rust. However,
    /// we want to mark this method as something, that should never be used without caution.
    unsafe fn route_frame_internal(&self, frame: Frame<V>, scope: BroadcastScope) -> Result<()>;

    /// <sup>⛔</sup>
    /// Message processor that is responsible for message signing and frame compatibility.
    fn processor_internal(&self) -> &FrameProcessor;
}

/// <sup>🔒</sup>
/// This trait is implemented by node API providers: synchronous and asynchronous.
///
/// 🔒 This trait is sealed 🔒
pub trait NodeApi<V: MaybeVersioned>: NodeApiInternal<V> {}

impl<V: MaybeVersioned> NodeApiInternal<V> for Unset {
    fn info(&self) -> &ConnectionInfo {
        ConnectionInfo::unknown()
    }

    unsafe fn route_frame_internal(&self, _: Frame<V>, _: BroadcastScope) -> Result<()> {
        unreachable!()
    }

    fn processor_internal(&self) -> &FrameProcessor {
        unreachable!()
    }
}
impl<V: MaybeVersioned> NodeApi<V> for Unset {}
