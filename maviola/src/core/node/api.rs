use crate::core::io::ConnectionInfo;
use crate::core::utils::Sealed;

use crate::prelude::*;

/// This trait is implemented by node state providers: synchronous and asynchronous.
pub trait NodeApi<V: MaybeVersioned + 'static>: Sealed {
    /// Provides information about connection.
    fn info(&self) -> &ConnectionInfo {
        &ConnectionInfo::Unknown
    }

    /// Returns `true` if node has active peers.
    fn has_peers(&self) -> bool;

    /// Send a MAVLink frame.
    fn send_frame(&self, frame: &Frame<V>) -> Result<()>;
}

/// Node without a defined API.
pub struct NoApi;
impl Sealed for NoApi {}
impl<V: MaybeVersioned + 'static> NodeApi<V> for NoApi {
    fn has_peers(&self) -> bool {
        unimplemented!()
    }

    fn send_frame(&self, _: &Frame<V>) -> Result<()> {
        unimplemented!()
    }
}
