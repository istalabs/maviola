use crate::core::io::OutgoingFrame;
use crate::protocol::Frame;
use crate::sync::io::Callback;

use crate::prelude::*;

#[cfg(doc)]
use crate::sync::io::{Channel, Connection};

/// <sup>[`sync`](crate::sync)</sup>
/// Sends outgoing frames to a [`Connection`] for processing.
///
/// Paired with [`OutgoingFrameHandler`], that usually is owned by a [`Channel`].
pub type OutgoingFrameSender<V> = mpmc::Sender<OutgoingFrame<V>>;
/// <sup>[`sync`](crate::sync)</sup>
/// Receives incoming frames from a [`Connection`].
///
/// Paired with [`IncomingFrameProducer`], that usually owned by a [`Channel`] and receives incoming
/// frames from the underlying transport.
pub type IncomingFrameReceiver<V> = mpmc::Receiver<(Frame<V>, Callback<V>)>;
/// <sup>[`sync`](crate::sync)</sup>
/// Handles outgoing frames, that were sent to [`Connection`] for processing.
///
/// Usually owned by a channels, that intercept outgoing frames and writes them to the underlying
/// transport. Paired with [`OutgoingFrameSender`] which is owned by [`Connection`].
pub type OutgoingFrameHandler<V> = mpmc::Receiver<OutgoingFrame<V>>;
/// <sup>[`sync`](crate::sync)</sup>
/// Produces incoming frames from the underlying transport.
///
/// Owned by a [`Channel`], that reads frames from the underlying transport and emits them to the
/// associated [`Connection`]. Paired with [`IncomingFrameReceiver`].
pub type IncomingFrameProducer<V> = mpmc::Sender<(Frame<V>, Callback<V>)>;
