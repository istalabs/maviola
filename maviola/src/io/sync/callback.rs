use std::sync::Arc;

use crate::io::broadcast::{BroadcastScope, OutgoingFrame};
use crate::io::sync::conn::FrameSender;
use crate::io::ChannelInfo;
use crate::utils::UniqueId;

use crate::prelude::*;

/// Callback object which caller receives upon each incoming frame.
///
/// You can control how to respond to incoming frames by choosing a channel to which response will
/// be broadcast:
///
/// * [`Callback::respond`] sends frame directly to the sender's channel.
/// * [`Callback::respond_others`] sends frame to all channels except the one from which original
///   frame was received.
/// * [`Callback::respond_all`] sends frame to all channels.
#[derive(Clone, Debug)]
pub struct Callback<V: MaybeVersioned> {
    pub(super) sender_id: UniqueId,
    pub(super) sender_info: Arc<ChannelInfo>,
    pub(super) broadcast_tx: FrameSender<V>,
}

impl<V: MaybeVersioned> Callback<V> {
    /// Information about sender's connection.
    pub fn info(&self) -> &ChannelInfo {
        self.sender_info.as_ref()
    }

    /// Respond directly to the peer which sent the [`Callback`].
    pub fn respond(&self, frame: &Frame<V>) -> Result<()> {
        self.broadcast_tx
            .send(OutgoingFrame::scoped(
                frame.clone(),
                BroadcastScope::Exact(self.sender_id),
            ))
            .map_err(Error::from)
    }

    /// Respond to all peers except the one which sent the initial frame.
    pub fn respond_others(&self, frame: &Frame<V>) -> Result<()> {
        self.broadcast_tx
            .send(OutgoingFrame::scoped(
                frame.clone(),
                BroadcastScope::Except(self.sender_id),
            ))
            .map_err(Error::from)
    }

    /// Respond to all peers including the one which has sent the initial frame.
    pub fn respond_all(&self, frame: &Frame<V>) -> Result<()> {
        self.broadcast_tx
            .send(OutgoingFrame::scoped(frame.clone(), BroadcastScope::All))
            .map_err(Error::from)
    }
}
