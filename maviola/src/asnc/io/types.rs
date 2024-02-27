use crate::asnc::io::{AsyncCallback, AsyncChannel, AsyncConnection};
use crate::core::io::OutgoingFrame;
use crate::protocol::Frame;

use crate::prelude::*;

/// <sup>[`async`](crate::asnc)</sup>
/// Producing part of channel that sends outgoing frames to a [`AsyncConnection`].
///
/// Paired with [`AsyncFrameSendHandler`] that usually is owned by a [`AsyncChannel`].
pub type AsyncFrameSender<V> = broadcast::Sender<OutgoingFrame<V>>;

/// <sup>[`async`](crate::asnc)</sup>
/// Receiver for incoming frames.
///
/// Paired with [`AsyncFrameProducer`] that usually owned by a [`AsyncChannel`] and receives incoming frames
/// from the underlying transport.
pub type AsyncFrameReceiver<V> = broadcast::Receiver<(Frame<V>, AsyncCallback<V>)>;

/// <sup>[`async`](crate::asnc)</sup>
/// Handles outgoing frames produced by [`AsyncConnection::send`].
///
/// Usually owned by channels which intercept outgoing frames and write them to the underlying
/// transport. Paired with [`AsyncFrameSender`] which is owned by [`AsyncConnection`].
pub type AsyncFrameSendHandler<V> = broadcast::Receiver<OutgoingFrame<V>>;

/// <sup>[`async`](crate::asnc)</sup>
/// Produces incoming frames.
///
/// Owned by a [`AsyncChannel`] that reads frames from the underlying transport and emits them to the
/// associated [`AsyncConnection`]. Paired with [`AsyncFrameReceiver`].
pub type AsyncFrameProducer<V> = broadcast::Sender<(Frame<V>, AsyncCallback<V>)>;
