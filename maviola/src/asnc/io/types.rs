use crate::asnc::io::{Callback, Channel, Connection};
use crate::core::io::OutgoingFrame;
use crate::protocol::Frame;

use crate::prelude::*;

/// <sup>[`async`](crate::asnc)</sup>
/// Producing part of channel that sends outgoing frames to a [`Connection`].
///
/// Paired with [`FrameSendHandler`] that usually is owned by a [`Channel`].
pub type FrameSender<V> = broadcast::Sender<OutgoingFrame<V>>;

/// <sup>[`async`](crate::asnc)</sup>
/// Receiver for incoming frames.
///
/// Paired with [`FrameProducer`] that usually owned by a [`Channel`] and receives incoming frames
/// from the underlying transport.
pub type FrameReceiver<V> = broadcast::Receiver<(Frame<V>, Callback<V>)>;

/// <sup>[`async`](crate::asnc)</sup>
/// Handles outgoing frames produced by [`Connection::send`].
///
/// Usually owned by channels which intercept outgoing frames and write them to the underlying
/// transport. Paired with [`FrameSender`] which is owned by [`Connection`].
pub type FrameSendHandler<V> = broadcast::Receiver<OutgoingFrame<V>>;

/// <sup>[`async`](crate::asnc)</sup>
/// Produces incoming frames.
///
/// Owned by a [`Channel`] that reads frames from the underlying transport and emits them to the
/// associated [`Connection`]. Paired with [`FrameReceiver`].
pub type FrameProducer<V> = broadcast::Sender<(Frame<V>, Callback<V>)>;
