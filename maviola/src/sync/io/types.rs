use crate::core::io::OutgoingFrame;
use crate::protocol::Frame;
use crate::sync::io::Callback;

use crate::prelude::*;

#[cfg(doc)]
use crate::sync::io::{Channel, Connection};

/// <sup>[`sync`](crate::sync)</sup>
/// Producing part of channel that sends outgoing frames to a [`Connection`].
///
/// Paired with [`FrameSendHandler`] that usually is owned by a [`Channel`].
pub type FrameSender<V> = mpmc::Sender<OutgoingFrame<V>>;
/// <sup>[`sync`](crate::sync)</sup>
/// Receiver for incoming frames.
///
/// Paired with [`FrameProducer`] that usually owned by a [`Channel`] and receives incoming frames
/// from the underlying transport.
pub type FrameReceiver<V> = mpmc::Receiver<(Frame<V>, Callback<V>)>;
/// <sup>[`sync`](crate::sync)</sup>
/// Handles outgoing frames produced by [`Connection::send`].
///
/// Usually owned by channels which intercept outgoing frames and write them to the underlying
/// transport. Paired with [`FrameSender`] which is owned by [`Connection`].
pub type FrameSendHandler<V> = mpmc::Receiver<OutgoingFrame<V>>;
/// <sup>[`sync`](crate::sync)</sup>
/// Produces incoming frames.
///
/// Owned by a [`Channel`] that reads frames from the underlying transport and emits them to the
/// associated [`Connection`]. Paired with [`FrameReceiver`].
pub type FrameProducer<V> = mpmc::Sender<(Frame<V>, Callback<V>)>;
