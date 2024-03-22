use crate::core::io::BroadcastScope;
use crate::core::utils::Sealed;
use crate::protocol::{DialectSpec, FrameProcessor};

use crate::prelude::*;

/// <sup>⛔</sup>
/// Internal API for sending frames.
///
/// 🔒 This trait is sealed and not exported to the library users 🔒
pub trait SendFrameInternal<V: MaybeVersioned>: Sealed {
    /// <sup>⛔</sup>
    /// Returns a reference to [`FrameProcessor`], that is responsible for message signing,
    /// compatibility and incompatibility flags, and custom message processing.
    fn processor(&self) -> &FrameProcessor;

    /// <sup>⛔</sup>
    /// Routes MAVLink frame without any changes.
    ///
    /// There is nothing particularly unsafe in this method in the sense of unsafe Rust. However,
    /// we want to mark this method as something, that should never be used without caution.
    unsafe fn route_frame_internal(&self, frame: Frame<V>, scope: BroadcastScope) -> Result<()>;
}

/// <sup>⛔</sup>
/// Internal API for sending messages.
///
/// 🔒 This trait is sealed and not exported to the library users 🔒
pub trait SendMessageInternal<V: MaybeVersioned>: Sealed {
    /// Endpoint specification.
    fn endpoint(&self) -> &Endpoint<V>;
}

/// <sup>🔒</sup>
/// Frame send API.
///
/// 🔒 This trait is sealed 🔒
pub trait SendFrame<V: MaybeVersioned>: SendFrameInternal<V> {
    /// Dialect specification.
    ///
    /// Default dialect is [`DefaultDialect`].
    #[inline]
    fn dialect(&self) -> &DialectSpec {
        self.processor().main_dialect()
    }

    /// Known dialects specifications.
    ///
    /// Node can perform frame validation against known dialects. However, automatic operations,
    /// like heartbeats, will use the main [`dialect`].
    ///
    /// Main dialect is always among the known dialects.
    ///
    /// [`dialect`]: Self::dialect
    #[inline(always)]
    fn known_dialects(&self) -> impl Iterator<Item = &DialectSpec> {
        self.processor().known_dialects()
    }

    /// Sends MAVLink [`Frame`].
    ///
    /// The [`Frame`] may be transformed according to frame processing configuration.
    ///
    /// To send MAVLink messages instead of raw frames, construct an [`Edge`] node and use
    /// [`send_versioned`] for node which is [`Versionless`] and [`send`] for [`Versioned`] nodes.
    /// In the latter case, message will be encoded according to MAVLink
    /// protocol version defined for a node.
    ///
    /// [`send`]: SendMessage::send
    /// [`send_versioned`]: SendVersionlessMessage::send_versioned
    /// [`Edge`]: crate::core::marker::Edge
    fn send_frame(&self, frame: &Frame<V>) -> Result<()> {
        let mut frame = frame.clone();
        self.processor().process_outgoing(&mut frame)?;
        unsafe { self.route_frame_internal(frame, BroadcastScope::All) }
    }

    /// Broadcasts MAVLink frame according to the specified broadcast `scope`.
    ///
    /// Using [`BroadcastScope::All`] is similar to just calling [`send_frame`].
    ///
    /// To broadcast MAVLink messages instead of raw frames, construct an [`Edge`] node and use
    /// [`broadcast_versioned`] for node which is [`Versionless`] and [`broadcast`] for
    /// [`Versioned`] nodes. In the latter case, message will be encoded according to MAVLink
    /// protocol version defined for a node.
    ///
    /// [`send_frame`]: Self::send_frame
    /// [`broadcast`]: SendMessage::broadcast
    /// [`broadcast_versioned`]: SendVersionlessMessage::broadcast_versioned
    /// [`Edge`]: crate::core::marker::Edge
    fn broadcast_frame(&self, frame: &Frame<V>, scope: BroadcastScope) -> Result<()> {
        let mut frame = frame.clone();
        self.processor().process_outgoing(&mut frame)?;
        unsafe { self.route_frame_internal(frame, scope) }
    }
}

/// <sup>🔒</sup>
///
/// Message sending API.
pub trait SendMessage<V: Versioned>: SendFrame<V> + SendMessageInternal<V> {
    /// Sends MAVLink message.
    ///
    /// The message will be encoded according to the node's dialect specification and MAVLink
    /// protocol version.
    ///
    /// If you want to send messages within different MAVLink protocols simultaneously, you have
    /// to construct a [`Versionless`] node and use [`Node::send_versioned`].
    fn send(&self, message: &impl Message) -> Result<()> {
        let frame = self.next_frame(message)?;
        self.send_frame(&frame)
    }

    /// Broadcasts MAVLink message according to the specified broadcast `scope`.
    ///
    /// The message will be encoded according to the node's dialect specification and MAVLink
    /// protocol version.
    ///
    /// Using [`BroadcastScope::All`] is similar to just calling [`send`].
    ///
    /// If you want to broadcast messages within different MAVLink protocols simultaneously, you
    /// have to construct a [`Versionless`] node and use [`Node::broadcast_versioned`].
    ///
    /// [`send`]: Self::send
    fn broadcast(&self, message: &impl Message, scope: BroadcastScope) -> Result<()> {
        let frame = self.next_frame(message)?;
        self.broadcast_frame(&frame, scope)
    }

    /// Creates a next frame from MAVLink message.
    ///
    /// If [`FrameSigner`] is set and the node has `MAVLink 2` protocol version, then frame will
    /// be signed according to the [`FrameSigner::outgoing`] strategy and filled with proper
    /// compatibility and incompatibility flags.
    fn next_frame(&self, message: &impl Message) -> Result<Frame<V>> {
        let mut frame = self.endpoint().next_frame(message)?;
        self.processor().process_new(&mut frame);
        Ok(frame)
    }
}

/// <sup>🔒</sup>
///
/// API for sending messages within version-agnostic channels.
pub trait SendVersionlessMessage:
    SendFrame<Versionless> + SendMessageInternal<Versionless>
{
    /// Sends MAVLink frame with a specified MAVLink protocol version.
    ///
    /// If you want to restrict MAVLink protocol to a particular version, construct a [`Versioned`]
    /// node and simply send messages by calling [`send`].
    ///
    /// [`send`]: SendMessage::send
    fn send_versioned<V: Versioned>(&self, message: &impl Message) -> Result<()> {
        let frame = self.next_frame_versioned::<V>(message)?;
        self.send_frame(&frame)
    }

    /// Broadcasts MAVLink frame with a specified MAVLink protocol version.
    ///
    /// Using [`BroadcastScope::All`] is similar to just calling [`send_versioned`].
    ///
    /// If you want to restrict MAVLink protocol to a particular version, construct a [`Versioned`]
    /// node and simply send messages by calling [`broadcast`].
    ///
    /// [`send_versioned`]: Self::send_versioned
    /// [`broadcast`]: SendMessage::broadcast
    fn broadcast_versioned<V: Versioned>(
        &self,
        message: &impl Message,
        scope: BroadcastScope,
    ) -> Result<()> {
        let frame = self.next_frame_versioned::<V>(message)?;
        self.broadcast_frame(&frame, scope)
    }

    /// Create a next frame from MAVLink message with a specified protocol version.
    ///
    /// After creation, the frame will be converted into a [`Versionless`] form.
    fn next_frame_versioned<V: Versioned>(
        &self,
        message: &impl Message,
    ) -> Result<Frame<Versionless>> {
        let mut frame = self.endpoint().next_frame::<V>(message)?;
        self.processor().process_new(&mut frame);
        Ok(frame)
    }
}
