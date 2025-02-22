use std::time::Duration;

use crate::asnc::consts::CONN_BROADCAST_CHAN_CAPACITY;
use crate::core::io::{IncomingFrame, OutgoingFrame};
use crate::core::utils::Closable;
#[cfg(feature = "unstable")]
use crate::error::TryRecvResult;
use crate::error::{RecvResult, RecvTimeoutResult, SendError, SendResult};

use crate::asnc::prelude::*;
use crate::prelude::*;

#[cfg(doc)]
#[cfg(feature = "unstable")]
use crate::asnc::io::{Channel, Connection};

/// <sup>[`async`](crate::asnc)</sup>
/// Sends outgoing frames to a [`Connection`] for processing.
///
/// Paired with [`OutgoingFrameHandler`], that usually is owned by a [`Channel`].
#[derive(Clone, Debug)]
pub struct OutgoingFrameSender<V: MaybeVersioned> {
    sender: mpmc::Sender<OutgoingFrame<V>>,
    state: Closable,
}

/// <sup>[`async`](crate::asnc)</sup>
/// Handles outgoing frames, that were sent to [`Connection`] for processing.
///
/// Usually owned by channels which intercept outgoing frames and write them to the underlying
/// transport. Paired with [`OutgoingFrameSender`] which is owned by [`Connection`].
#[derive(Clone, Debug)]
pub struct OutgoingFrameHandler<V: MaybeVersioned> {
    receiver: mpmc::Receiver<OutgoingFrame<V>>,
}

/// <sup>[`async`](crate::asnc)</sup>
/// Produces incoming frames from the underlying transport.
///
/// Owned by a [`Channel`], that reads frames from the underlying transport and emits them to the
/// associated [`Connection`]. Paired with [`IncomingFrameReceiver`].
#[derive(Clone, Debug)]
pub struct IncomingFrameProducer<V: MaybeVersioned> {
    sender: mpmc::Sender<IncomingFrame<V>>,
}

/// <sup>[`async`](crate::asnc)</sup>
/// Receives incoming frames from a [`Connection`].
///
/// Paired with [`IncomingFrameProducer`], that usually owned by a [`Channel`] and receives
/// incoming frames from the underlying transport.
#[derive(Clone, Debug)]
pub struct IncomingFrameReceiver<V: MaybeVersioned> {
    receiver: mpmc::Receiver<IncomingFrame<V>>,
}

/// Creates outgoing frames channel that is responsible for passing frames from API to connection.
pub fn outgoing_channel<V: MaybeVersioned>(
    state: Closable,
) -> (OutgoingFrameSender<V>, OutgoingFrameHandler<V>) {
    let (tx, rx) = mpmc::channel(CONN_BROADCAST_CHAN_CAPACITY);
    (
        OutgoingFrameSender::new(tx, state),
        OutgoingFrameHandler::new(rx),
    )
}

/// Creates incoming frames channel that is responsible for passing frames from connection to API.
pub fn incoming_channel<V: MaybeVersioned>() -> (IncomingFrameProducer<V>, IncomingFrameReceiver<V>)
{
    let (tx, rx) = mpmc::channel(CONN_BROADCAST_CHAN_CAPACITY);
    (
        IncomingFrameProducer::new(tx),
        IncomingFrameReceiver::new(rx),
    )
}

impl<V: MaybeVersioned> OutgoingFrameSender<V> {
    fn new(sender: mpmc::Sender<OutgoingFrame<V>>, state: Closable) -> Self {
        Self { sender, state }
    }

    /// Sends frame to all possible channels.
    #[inline(always)]
    pub fn send(&self, frame: Frame<V>) -> SendResult<OutgoingFrame<V>> {
        self.send_raw(OutgoingFrame::new(frame))
    }

    /// Sends outgoing frame with specified routing.
    pub fn send_raw(&self, frame: OutgoingFrame<V>) -> SendResult<OutgoingFrame<V>> {
        if self.state.is_closed() {
            return Err(SendError(frame));
        }

        self.sender.send(frame)
    }
}

impl<V: MaybeVersioned> OutgoingFrameHandler<V> {
    fn new(receiver: mpmc::Receiver<OutgoingFrame<V>>) -> Self {
        Self { receiver }
    }

    /// Receives outgoing frame blocking until either frame is received or channel is closed.
    #[inline(always)]
    pub async fn recv(&mut self) -> RecvResult<OutgoingFrame<V>> {
        self.receiver.recv().await
    }

    /// Receives outgoing frame with timeout.
    #[inline(always)]
    pub async fn recv_timeout(&mut self, timeout: Duration) -> RecvTimeoutResult<OutgoingFrame<V>> {
        self.receiver.recv_timeout(timeout).await
    }
}

impl<V: MaybeVersioned> IncomingFrameProducer<V> {
    fn new(sender: mpmc::Sender<IncomingFrame<V>>) -> Self {
        Self { sender }
    }

    /// Sends incoming frame.
    #[allow(clippy::result_large_err)]
    pub fn send(&self, frame: IncomingFrame<V>) -> SendResult<IncomingFrame<V>> {
        self.sender.send(frame)
    }
}

impl<V: MaybeVersioned> IncomingFrameReceiver<V> {
    fn new(receiver: mpmc::Receiver<IncomingFrame<V>>) -> Self {
        Self { receiver }
    }

    /// Receives incoming frame blocking until either frame is received or channel is closed.
    #[inline(always)]
    #[cfg(feature = "unstable")]
    pub async fn recv(&mut self) -> RecvResult<IncomingFrame<V>> {
        self.receiver.recv().await
    }

    /// Receives incoming frame with timeout.
    #[inline(always)]
    pub async fn recv_timeout(&mut self, timeout: Duration) -> RecvTimeoutResult<IncomingFrame<V>> {
        self.receiver.recv_timeout(timeout).await
    }

    /// Attempts to receive incoming frame without blocking.
    #[inline(always)]
    #[cfg(feature = "unstable")]
    pub fn try_recv(&mut self) -> TryRecvResult<IncomingFrame<V>> {
        self.receiver.try_recv()
    }
}
