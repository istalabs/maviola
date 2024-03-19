use crate::core::io::{BroadcastScope, ChannelId, ChannelInfo, ConnectionId, OutgoingFrame};
use crate::core::utils::Sealed;

use crate::prelude::*;

/// <sup>🔒</sup>
/// Internal callback API.
pub trait CallbackApiInternal<V: MaybeVersioned>: Sealed {
    /// <sup>🔒</sup>
    /// Send outgoing frame internally.
    fn send_internal(&self, frame: OutgoingFrame<V>) -> Result<()>;

    /// <sup>🔒</sup>
    /// Process frame internally according to the defined rules.
    fn process_frame(&self, frame: &Frame<V>) -> Result<Frame<V>>;
}

/// <sup>🔒</sup>
/// Callback API.
///
/// ⚠ This trait is sealed ⚠
///
/// This trait is implemented by callbacks for synchronous and asynchronous API.
pub trait CallbackApi<V: MaybeVersioned>: CallbackApiInternal<V> {
    /// Information about sender's channel.
    fn info(&self) -> &ChannelInfo;

    /// Identifier of a sender's channel.
    #[inline(always)]
    fn channel_id(&self) -> &ChannelId {
        self.info().id()
    }

    /// Identifier of a sender's connection.
    #[inline(always)]
    fn connection_id(&self) -> &ConnectionId {
        self.info().connection_id()
    }

    /// Send frame to all channels including the one which has sent the original frame.
    fn send(&self, frame: &Frame<V>) -> Result<()> {
        let frame = self.process_frame(frame)?;
        self.send_internal(OutgoingFrame::scoped(frame, BroadcastScope::All))
    }

    /// Respond directly to the channel which sent the original frame.
    fn respond(&self, frame: &Frame<V>) -> Result<()> {
        let frame = self.process_frame(frame)?;
        self.send_internal(OutgoingFrame::scoped(
            frame,
            BroadcastScope::ExactChannel(*self.channel_id()),
        ))
    }

    /// Broadcast to all channels except the one which sent the original frame.
    fn broadcast(&self, frame: &Frame<V>) -> Result<()> {
        let frame = self.process_frame(frame)?;
        self.send_internal(OutgoingFrame::scoped(
            frame,
            BroadcastScope::ExceptChannel(*self.channel_id()),
        ))
    }

    /// Broadcast to all channels within its own connection except the one which sent the original
    /// frame.
    ///
    /// This is similar to [`Self::broadcast`], except it reduces broadcast scope to the
    /// connection from which the original frame was received.
    fn broadcast_within(&self, frame: &Frame<V>) -> Result<()> {
        let frame = self.process_frame(frame)?;
        self.send_internal(OutgoingFrame::scoped(
            frame,
            BroadcastScope::ExceptChannelWithin(*self.channel_id()),
        ))
    }

    /// Broadcast frame to all connections except the one which sent this frame.
    fn broadcast_except(&self, frame: &Frame<V>) -> Result<()> {
        let frame = self.process_frame(frame)?;
        self.send_internal(OutgoingFrame::scoped(
            frame,
            BroadcastScope::ExceptConnection(*self.connection_id()),
        ))
    }

    /// Forward a frame to all channels of a connection with specified `connection_id`.
    fn forward(&self, frame: &Frame<V>, connection_id: ConnectionId) -> Result<()> {
        let frame = self.process_frame(frame)?;
        self.send_internal(OutgoingFrame::scoped(
            frame,
            BroadcastScope::ExactConnection(connection_id),
        ))
    }
}
