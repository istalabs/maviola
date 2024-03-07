use crate::asnc::io::Callback;
use crate::core::io::OutgoingFrame;
use crate::protocol::Frame;

use crate::prelude::*;

#[cfg(doc)]
use crate::asnc::io::{Channel, Connection};

/// <sup>[`async`](crate::asnc)</sup>
/// Sends outgoing frames to a [`Connection`] for processing.
///
/// Paired with [`OutgoingFrameHandler`], that usually is owned by a [`Channel`].
pub type OutgoingFrameSender<V> = broadcast::Sender<OutgoingFrame<V>>;

/// <sup>[`async`](crate::asnc)</sup>
/// Receives incoming frames from a [`Connection`].
///
/// Paired with [`IncomingFrameProducer`], that usually owned by a [`Channel`] and receives
/// incoming frames from the underlying transport.
pub type IncomingFrameReceiver<V> = broadcast::Receiver<(Frame<V>, Callback<V>)>;

/// <sup>[`async`](crate::asnc)</sup>
/// Handles outgoing frames, that were sent to [`Connection`] for processing.
///
/// Usually owned by channels which intercept outgoing frames and write them to the underlying
/// transport. Paired with [`OutgoingFrameSender`] which is owned by [`Connection`].
pub type OutgoingFrameHandler<V> = broadcast::Receiver<OutgoingFrame<V>>;

/// <sup>[`async`](crate::asnc)</sup>
/// Produces incoming frames from the underlying transport.
///
/// Owned by a [`Channel`], that reads frames from the underlying transport and emits them to the
/// associated [`Connection`]. Paired with [`IncomingFrameReceiver`].
pub type IncomingFrameProducer<V> = broadcast::Sender<(Frame<V>, Callback<V>)>;
