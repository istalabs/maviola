use std::sync::Arc;

use crate::core::io::ChannelInfo;
use crate::core::io::OutgoingFrame;
use crate::core::node::CallbackApiInternal;
use crate::core::utils::Sealed;
use crate::protocol::FrameProcessor;
use crate::sync::node::api::FrameSender;

use crate::prelude::*;

/// <sup>[`sync`](crate::sync)</sup>
/// Callback object which caller receives upon each incoming frame.
///
/// You can control how to respond to incoming frames by choosing a channel to which response will
/// be broadcast (import [`CallbackApi`] or use [`prelude`](crate::prelude) to access these
/// methods):
///
/// * [`Callback::send`] sends frame to all possible channels.
/// * [`Callback::respond`] sends frame directly to the sender's channel.
/// * [`Callback::broadcast`] broadcast frame to all channels except the one from which the
///   original frame was received.
/// * [`Callback::broadcast_within`] broadcast frame to all channels within sender's connection
///   except the channel from which the original frame was received.
/// * [`Callback::broadcast_except`] broadcast frame to all connections except the one which sent
///   this frame.
/// * [`Callback::forward`] forward a frame to all channels of a specific connection.
#[derive(Clone, Debug)]
pub struct Callback<V: MaybeVersioned> {
    channel_info: ChannelInfo,
    sender: FrameSender<V>,
}

impl<V: MaybeVersioned> Callback<V> {
    pub(super) fn new(channel_info: ChannelInfo, sender: FrameSender<V>) -> Self {
        Self {
            channel_info,
            sender,
        }
    }

    pub(in crate::sync) fn set_processor(&mut self, processor: Arc<FrameProcessor>) {
        self.sender.set_processor(processor);
    }
}

impl<V: MaybeVersioned> Sealed for Callback<V> {}

impl<V: MaybeVersioned> CallbackApiInternal<V> for Callback<V> {
    fn send_internal(&self, frame: OutgoingFrame<V>) -> Result<()> {
        self.sender.send_raw(frame).map_err(Error::from).map(|_| ())
    }

    fn process_frame(&self, frame: &Frame<V>) -> Result<Frame<V>> {
        let mut frame = frame.clone();
        self.sender.processor().process_outgoing(&mut frame)?;
        Ok(frame)
    }
}

impl<V: MaybeVersioned> CallbackApi<V> for Callback<V> {
    fn info(&self) -> &ChannelInfo {
        &self.channel_info
    }
}

impl<V: MaybeVersioned> From<Callback<V>> for ChannelInfo {
    fn from(value: Callback<V>) -> Self {
        value.channel_info
    }
}
