use std::sync::Arc;

use crate::core::io::ChannelInfo;
use crate::core::io::{BroadcastScope, OutgoingFrame};
use crate::core::utils::UniqueId;
use crate::sync::io::OutgoingFrameSender;

use crate::prelude::*;

/// <sup>[`sync`](crate::sync)</sup>
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
    sender_id: UniqueId,
    sender_info: Arc<ChannelInfo>,
    sender: OutgoingFrameSender<V>,
    signer: Option<Arc<MessageSigner>>,
}

impl<V: MaybeVersioned> Callback<V> {
    /// Information about sender's connection.
    pub fn info(&self) -> &ChannelInfo {
        self.sender_info.as_ref()
    }

    /// Respond directly to the peer which sent the [`Callback`].
    pub fn respond(&self, frame: &Frame<V>) -> Result<()> {
        let frame = self.process_outgoing_frame(frame)?;
        self.send_internal(OutgoingFrame::scoped(
            frame,
            BroadcastScope::Exact(self.sender_id),
        ))
    }

    /// Respond to all peers except the one which sent the initial frame.
    pub fn respond_others(&self, frame: &Frame<V>) -> Result<()> {
        let frame = self.process_outgoing_frame(frame)?;
        self.send_internal(OutgoingFrame::scoped(
            frame,
            BroadcastScope::Except(self.sender_id),
        ))
    }

    /// Respond to all peers including the one which has sent the initial frame.
    pub fn respond_all(&self, frame: &Frame<V>) -> Result<()> {
        let frame = self.process_outgoing_frame(frame)?;
        self.send_internal(OutgoingFrame::scoped(frame, BroadcastScope::All))
    }

    pub(super) fn new(
        id: UniqueId,
        info: Arc<ChannelInfo>,
        sender: OutgoingFrameSender<V>,
    ) -> Self {
        Self {
            sender_id: id,
            sender_info: info,
            sender,
            signer: None,
        }
    }

    pub(in crate::sync) fn set_signer(&mut self, signer: Option<Arc<MessageSigner>>) {
        self.signer = signer;
    }

    fn process_outgoing_frame(&self, frame: &Frame<V>) -> Result<Frame<V>> {
        let mut frame = frame.clone();
        if let Some(signer) = &self.signer {
            signer.process_outgoing(&mut frame)?;
        }
        Ok(frame)
    }

    fn send_internal(&self, frame: OutgoingFrame<V>) -> Result<()> {
        self.sender.send(frame).map_err(Error::from)
    }
}
