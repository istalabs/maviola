use std::sync::Arc;

use mavio::protocol::MaybeVersioned;
use mavio::Frame;

use crate::io::broadcast::{BroadcastScope, OutgoingFrame};
use crate::io::PeerConnectionInfo;
use crate::utils::UniqueId;

use crate::prelude::*;

/// Response object which caller receives upon each incoming frame.
#[derive(Clone, Debug)]
pub struct AsyncResponse<V: MaybeVersioned> {
    pub(super) sender_id: UniqueId,
    pub(super) sender_info: Arc<PeerConnectionInfo>,
    pub(super) broadcast_tx: broadcast::Sender<OutgoingFrame<V>>,
}

impl<V: MaybeVersioned> AsyncResponse<V> {
    /// Information about sender's connection.
    pub fn info(&self) -> &PeerConnectionInfo {
        self.sender_info.as_ref()
    }

    /// Respond directly to the peer which sent the [`AsyncResponse`].
    pub fn respond(&self, frame: &Frame<V>) -> Result<usize> {
        self.broadcast_tx
            .send(OutgoingFrame::scoped(
                frame.clone(),
                BroadcastScope::Exact(self.sender_id),
            ))
            .map_err(Error::from)
    }

    /// Respond to all peers except the one which sent the initial frame.
    pub fn respond_others(&self, frame: &Frame<V>) -> Result<usize> {
        self.broadcast_tx
            .send(OutgoingFrame::scoped(
                frame.clone(),
                BroadcastScope::Except(self.sender_id),
            ))
            .map_err(Error::from)
    }

    /// Respond to all peers including the one which has sent the initial frame.
    pub fn respond_all(&self, frame: &Frame<V>) -> Result<usize> {
        self.broadcast_tx
            .send(OutgoingFrame::scoped(frame.clone(), BroadcastScope::All))
            .map_err(Error::from)
    }
}
