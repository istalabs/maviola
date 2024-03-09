//! # <sup>`⍚` | [`asnc`](crate::asnc)</sup> Multiple producers / multiple consumers broadcast channel

use std::fmt::{Debug, Formatter};
use std::time::Duration;

use tokio::sync::broadcast;

use crate::core::error::{RecvError, RecvTimeoutError, SendError, TryRecvError};

/// <sup>`⍚` | [`asnc`](crate::asnc)</sup>
/// MPMC sender.
///
/// Behaves almost identical to [`broadcast::Sender`]. The latter [`broadcast`] sender can be
/// obtained through the [`Sender::into_inner`] method.
#[derive(Clone)]
pub struct Sender<T> {
    inner: broadcast::Sender<T>,
}

/// <sup>`⍚` | [`asnc`](crate::asnc)</sup>
/// MPMC receiver.
///
/// Behaves almost identical to [`broadcast::Receiver`] except that it can be cloned. The underlying
/// [`broadcast`] receiver can be obtained through the [`Receiver::into_inner`] method.
///
/// Cloning does not create an identical receiver. Instead, it creates a new receiver, that listens
/// to messages, that have been sent around the time of its creation.
///
/// Each cloned receiver will receive its own message.
pub struct Receiver<T> {
    inner: broadcast::Receiver<T>,
}

impl<T> Sender<T> {
    /// Attempts to send a value on this channel, returning it back if it could
    /// not be sent.
    ///
    /// Behaves identical to [`broadcast::Sender::send`], but returns [`SendError`].
    pub fn send(&self, value: T) -> Result<(), SendError<T>> {
        self.inner.send(value).map_err(SendError::from).map(|_| ())
    }

    /// Returns inner [`broadcast::Sender`].
    #[allow(dead_code)]
    pub fn into_inner(self) -> broadcast::Sender<T> {
        self.inner
    }
}

impl<T: Clone> Receiver<T> {
    /// Attempts to wait for a value on this receiver, returning an error if the
    /// corresponding channel has hung up.
    ///
    /// Behaves identical to [`broadcast::Receiver::recv`] but returns [`RecvError`].
    pub async fn recv(&mut self) -> Result<T, RecvError> {
        self.inner.recv().await.map_err(RecvError::from)
    }

    /// Attempts to wait for a value on this receiver, returning an error if the
    /// corresponding channel has hung up, or if it waits more than `timeout`.
    ///
    /// Behaves similar to [`broadcast::Receiver::recv`] but returns [`RecvTimeoutError`] and stops,
    /// when deadline is reached.
    pub async fn recv_timeout(&mut self, timeout: Duration) -> Result<T, RecvTimeoutError> {
        match tokio::time::timeout(timeout, self.inner.recv()).await {
            Ok(result) => match result {
                Ok(value) => Ok(value),
                Err(err) => Err(match err {
                    broadcast::error::RecvError::Lagged(n) => RecvTimeoutError::Lagged(n),
                    broadcast::error::RecvError::Closed => RecvTimeoutError::Disconnected,
                }),
            },
            Err(_) => Err(RecvTimeoutError::Timeout),
        }
    }

    /// Attempts to return a pending value on this receiver without blocking.
    ///
    /// Behaves identical to [`broadcast::Receiver::try_recv`] but returns [`TryRecvError`].
    pub fn try_recv(&mut self) -> Result<T, TryRecvError> {
        self.inner.try_recv().map_err(TryRecvError::from)
    }

    /// Creates a new receiver subscribed to the channel.
    pub fn resubscribe(&self) -> Receiver<T> {
        Self {
            inner: self.inner.resubscribe(),
        }
    }

    /// Returns inner [`broadcast::Receiver`].
    #[allow(dead_code)]
    pub fn into_inner(self) -> broadcast::Receiver<T> {
        self.inner
    }
}

unsafe impl<T: Send> Send for Sender<T> {}
unsafe impl<T: Send> Sync for Sender<T> {}

unsafe impl<T: Send> Send for Receiver<T> {}
unsafe impl<T: Send> Sync for Receiver<T> {}

impl<T> Debug for Sender<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sender").finish_non_exhaustive()
    }
}

impl<T: Clone> Debug for Receiver<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Receiver").finish_non_exhaustive()
    }
}

impl<T: Clone> Clone for Receiver<T> {
    fn clone(&self) -> Self {
        self.resubscribe()
    }
}

/// Create a bounded, multi-producer, multi-consumer channel where each sent
/// value is broadcasted to all active receivers.
///
/// **Note:** The actual capacity may be greater than the provided `capacity`.
///
/// Similar to [`broadcast::channel`], but returns [`Sender`] / [`Receiver`] pair of wrappers.
///
/// # Panics
///
/// This will panic if `capacity` is equal to `0` or larger
/// than `usize::MAX / 2`.
pub fn channel<T: Clone>(capacity: usize) -> (Sender<T>, Receiver<T>) {
    let (tx, rx) = broadcast::channel(capacity);
    let sender = Sender { inner: tx };
    let receiver = Receiver { inner: rx };
    (sender, receiver)
}
