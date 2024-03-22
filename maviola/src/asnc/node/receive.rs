use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use tokio_stream::Stream;

use crate::core::utils::Sealed;
use crate::error::{RecvResult, RecvTimeoutError, RecvTimeoutResult, TryRecvError, TryRecvResult};
use crate::protocol::Behold;

use crate::asnc::prelude::*;
use crate::prelude::*;

/// <sup>🔒</sup>
/// Synchronous API for receiving node events.
///
/// 🔒 This trait is sealed 🔒
#[async_trait]
pub trait ReceiveEvent<V: MaybeVersioned>: Sealed {
    /// <sup>[`async`](crate::asnc)</sup>
    /// Receives the next node [`Event`].
    ///
    /// Blocks until event received.
    async fn recv(&mut self) -> RecvResult<Event<V>>;

    /// <sup>[`async`](crate::asnc)</sup>
    /// Attempts to receive the next node [`Event`] within a `timeout`.
    ///
    /// Blocks until event received or deadline is reached.
    async fn recv_timeout(&mut self, timeout: Duration) -> RecvTimeoutResult<Event<V>>;

    /// <sup>[`async`](crate::asnc)</sup>
    /// Attempts to receive MAVLink [`Event`] without blocking.
    fn try_recv(&mut self) -> TryRecvResult<Event<V>>;

    /// <sup>[`async`](crate::asnc)</sup>
    /// Subscribes to node events.
    ///
    /// Blocks while the underlying node is active.
    ///
    /// If you are interested only in valid incoming frames, use [`frames`], [`recv_frame`],
    /// [`recv_frame_timeout`], or [`try_recv_frame`] instead.
    ///
    /// [`recv_frame`]: ReceiveFrame::recv_frame
    /// [`recv_frame_timeout`]: ReceiveFrame::recv_frame_timeout
    /// [`try_recv_frame`]: ReceiveFrame::try_recv_frame
    /// [`frames`]: ReceiveFrame::frames
    fn events(&self) -> Behold<impl Stream<Item = Event<V>>>;
}

/// <sup>🔒</sup>
/// Synchronous API for receiving valid MAVLink frames.
///
/// 🔒 This trait is sealed 🔒
#[async_trait]
pub trait ReceiveFrame<V: MaybeVersioned>: ReceiveEvent<V> {
    /// <sup>[`async`](crate::asnc)</sup>
    /// Receives the next frame. Blocks until valid frame received or channel is closed.
    ///
    /// If you want to block until the next frame within a timeout, use [`recv_frame_timeout`].
    /// If you want to check for the next frame without blocking, use [`try_recv_frame`].
    ///
    /// **⚠** This method skips all invalid frames. If you are interested in such frames, use
    /// [`events`] or [`recv`] instead to receive [`Event::Invalid`] event that contain invalid
    /// frame with the corresponding error.
    ///
    /// [`recv_frame_timeout`]: Self::recv_frame_timeout
    /// [`try_recv_frame`]: Self::try_recv_frame
    /// [`events`]: ReceiveEvent::events
    /// [`recv`]: ReceiveEvent::recv
    async fn recv_frame(&mut self) -> RecvResult<(Frame<V>, Callback<V>)> {
        loop {
            match self.recv().await {
                Ok(Event::Frame(frame, callback)) => {
                    return Ok((frame, callback));
                }
                Ok(_) => continue,
                Err(err) => return Err(err),
            }
        }
    }

    /// <sup>[`async`](crate::asnc)</sup>
    /// Attempts ot receives the next frame until the timeout is reached. Blocks until valid frame
    /// received, deadline is reached, or channel is closed.
    ///
    /// If you want to block until the next frame is received, use [`recv_frame`].
    /// If you want to check for the next frame without blocking, use [`try_recv_frame`].
    ///
    /// **⚠** This method skips all invalid frames. If you are interested in such frames, use
    /// [`events`] or [`recv_timeout`] instead to receive [`Event::Invalid`] event that contains
    /// invalid frame with the corresponding error.
    ///
    /// [`recv_frame`]: Self::recv_frame
    /// [`try_recv_frame`]: Self::try_recv_frame
    /// [`events`]: ReceiveEvent::events
    /// [`recv_timeout`]: ReceiveEvent::recv_timeout
    async fn recv_frame_timeout(
        &mut self,
        timeout: Duration,
    ) -> RecvTimeoutResult<(Frame<V>, Callback<V>)> {
        let start = SystemTime::now();
        let mut current_timeout = timeout;

        loop {
            match self.recv_timeout(current_timeout).await {
                Ok(Event::Frame(frame, callback)) => {
                    return Ok((frame, callback));
                }
                Ok(_) => {
                    let since_start =
                        if let Ok(since_start) = SystemTime::now().duration_since(start) {
                            since_start
                        } else {
                            continue;
                        };

                    if let Some(new_timeout) = timeout.checked_sub(since_start) {
                        current_timeout = new_timeout;
                    } else {
                        return Err(RecvTimeoutError::Timeout);
                    }
                }
                Err(err) => return Err(err),
            }
        }
    }

    /// <sup>[`async`](crate::asnc)</sup>
    /// Attempts to receive the next valid frame. Returns immediately if channel is empty.
    ///
    /// If you want to block until the next frame within a timeout, use [`recv_frame_timeout`].
    /// If you want to block until the next frame is received, use [`recv_frame`].
    ///
    /// **⚠** This method skips all invalid frames. If you are interested in such frames, use
    /// [`events`] or [`try_recv`] instead to receive [`Event::Invalid`] event that contains invalid
    /// frame with the corresponding error.
    ///
    /// [`recv_frame`]: Self::recv_frame
    /// [`recv_frame_timeout`]: Self::recv_frame_timeout
    /// [`events`]: ReceiveEvent::events
    /// [`try_recv`]: ReceiveEvent::try_recv
    fn try_recv_frame(&mut self) -> TryRecvResult<(Frame<V>, Callback<V>)> {
        match self.try_recv() {
            Ok(Event::Frame(frame, callback)) => {
                return Ok((frame, callback));
            }
            Ok(_) => Err(TryRecvError::Empty),
            Err(err) => return Err(err),
        }
    }

    /// <sup>[`async`](crate::asnc)</sup>
    /// Subscribes to valid MAVLink frames.
    ///
    /// Blocks while the underlying node is active.
    ///
    /// **⚠** This method skips all invalid frames. If you are interested in such frames, use
    /// [`events`], [`recv`], [`recv_timeout`], or [`try_recv`] instead to receive
    /// [`Event::Invalid`] event that contains invalid frame with the corresponding error.
    ///
    /// [`recv`]: ReceiveEvent::recv
    /// [`recv_timeout`]: ReceiveEvent::recv_timeout
    /// [`try_recv`]: ReceiveEvent::try_recv
    /// [`events`]: ReceiveEvent::events
    fn frames(&self) -> Behold<impl Stream<Item = (Frame<V>, Callback<V>)>> {
        Behold::new(self.events().unwrap().filter_map(|event| match event {
            Event::Frame(frame, callback) => Some((frame, callback)),
            _ => None,
        }))
    }
}
