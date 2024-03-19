use crate::core::io::{BroadcastScope, ConnectionInfo};
use crate::core::utils::Sealed;
use crate::protocol::FrameProcessor;

use crate::prelude::*;

/// <sup>🔒</sup>
/// Internal node API.
pub trait NodeApiInternal<V: MaybeVersioned>: Sealed {
    /// Provides information about connection.
    fn info(&self) -> &ConnectionInfo;

    /// Send a MAVLink frame.
    fn send_frame(&self, frame: &Frame<V>) -> Result<()>;

    /// Route MAVLink frame.
    fn route_frame(&self, frame: &Frame<V>, scope: BroadcastScope) -> Result<()>;

    /// Message processor that is responsible for message signing and frame compatibility.
    fn processor(&self) -> &FrameProcessor;
}

/// <sup>🔒</sup>
/// This trait is implemented by node API providers: synchronous and asynchronous.
///
/// ⚠ This trait is sealed ⚠
pub trait NodeApi<V: MaybeVersioned>: NodeApiInternal<V> {}

/// Node without a defined API.
pub struct NoApi;
impl Sealed for NoApi {}
impl<V: MaybeVersioned> NodeApiInternal<V> for NoApi {
    fn info(&self) -> &ConnectionInfo {
        ConnectionInfo::unknown()
    }

    fn send_frame(&self, _: &Frame<V>) -> Result<()> {
        unreachable!()
    }

    fn route_frame(&self, _: &Frame<V>, _: BroadcastScope) -> Result<()> {
        unreachable!()
    }

    fn processor(&self) -> &FrameProcessor {
        unreachable!()
    }
}
impl<V: MaybeVersioned> NodeApi<V> for NoApi {}
