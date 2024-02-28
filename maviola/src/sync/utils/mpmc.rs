//! # <sup>`⍚` | [`sync`](crate::sync)</sup> Multiple producers / multiple consumers broadcast channel
//!
//! This module provides MPMC primitives which allow to broadcast messages over the channel from
//! multiple producers to multiple consumers:
//!
//! * [Sender]
//! * [Receiver]
//!
//! Similar to [`mpsc`], an MPMC channel is created by [`channel`] method.
//!
//! A [`Sender`] is used to broadcast messages to single or multiple instances of [`Receiver`].
//! These primitives behave almost identical to their [`mpsc`] counterparts, except that
//! [`Receiver`] can be cloned. A cloned receiver becomes an independent listener for channel's
//! messages.
//!
//! # Examples
//!
//! ```rust
//! # #[cfg(feature = "unstable")]{
//! use maviola::sync::utils::mpmc;
//!
//! let (tx_1, rx_1) = mpmc::channel();
//! let tx_2 = tx_1.clone();
//! let rx_2 = rx_1.clone();
//!
//! tx_1.send(1).unwrap();
//! tx_2.send(2).unwrap();
//!
//! assert_eq!(rx_1.recv().unwrap(), 1);
//! assert_eq!(rx_2.recv().unwrap(), 1);
//!
//! assert_eq!(rx_1.recv().unwrap(), 2);
//! assert_eq!(rx_2.recv().unwrap(), 2);
//! # }
//! ```

use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::sync::{mpsc, Arc, RwLock};
use std::thread;

use crate::core::utils::{Closable, Closer, UniqueId};

/// <sup>`⍚` | [`sync`](crate::sync)</sup>
/// MPMC sender.
///
/// Behaves almost identical to [`mpsc::Sender`]. The latter [`mpsc`] sender can be obtained through
/// the [`Sender::into_inner`] method.
#[derive(Clone, Debug)]
pub struct Sender<T> {
    inner: mpsc::Sender<T>,
    state: Closable,
}

unsafe impl<T: Send> Send for Sender<T> {}
unsafe impl<T: Sync> Sync for Sender<T> {}

impl<T> Sender<T> {
    /// Attempts to send a value on this channel, returning it back if it could
    /// not be sent.
    ///
    /// Behaves identical to [`mpsc::Sender::send`].
    pub fn send(&self, value: T) -> Result<(), mpsc::SendError<T>> {
        if self.state.is_closed() {
            return Err(mpsc::SendError(value));
        }

        self.inner.send(value)
    }

    /// Returns inner [`mpsc::Sender`].
    ///
    /// # Limitation
    ///
    /// Once inner sender has been obtained, it is no longer guaranteed that messages it sends will
    /// be consumed by at least one receiver. This may happen if the last receiver becomes
    /// disconnected right before or slightly after the message was sent.
    #[must_use]
    pub fn into_inner(self) -> mpsc::Sender<T> {
        self.inner
    }
}

/// <sup>`⍚` | [`sync`](crate::sync)</sup>
/// MPMC receiver.
///
/// Behaves almost identical to [`mpsc::Receiver`] except that it can be cloned. The underlying
/// [`mpsc`] receiver can be obtained through the [`Receiver::into_inner`] method.
///
/// Each cloned receiver will receive its own message.
pub struct Receiver<T: Clone + Sync + Send + 'static> {
    inner: mpsc::Receiver<T>,
    guard: RecvGuard<T>,
}

impl<T: Clone + Sync + Send + 'static> Debug for Receiver<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Receiver")
            .field("inner", &self.inner)
            .finish_non_exhaustive()
    }
}

impl<T: Clone + Sync + Send + 'static> Receiver<T> {
    /// Attempts to wait for a value on this receiver, returning an error if the
    /// corresponding channel has hung up.
    ///
    /// Behaves identical to [`mpsc::Receiver::recv`].
    pub fn recv(&self) -> Result<T, mpsc::RecvError> {
        self.inner.recv()
    }

    /// Attempts to return a pending value on this receiver without blocking.
    ///
    /// Behaves identical to [`mpsc::Receiver::try_recv`].
    pub fn try_recv(&self) -> Result<T, mpsc::TryRecvError> {
        self.inner.try_recv()
    }

    /// Returns inner [`mpsc::Receiver`].
    ///
    /// Returns inner receiver and [`RecvGuard`]. When guard is dropped, the receiver will be
    /// disconnected from the bus.
    ///
    /// # Usage
    ///
    /// Guard is present, the receiver can accept messages:
    ///
    /// ```rust
    /// # #[cfg(feature = "unstable")]{
    /// use std::thread;
    /// use std::sync::mpsc;
    /// use maviola::sync::utils::mpmc;
    ///
    /// let (tx, rx) = mpmc::channel();
    /// let (rx_inner, _guard) = rx.into_inner();
    ///
    /// let handler = thread::spawn(move || -> Result<(), mpsc::RecvError> { rx_inner.recv() });
    ///
    /// assert!(tx.send(()).is_ok());
    /// assert!(handler.join().unwrap().is_ok());
    /// # }
    /// ```
    ///
    /// Guard is dropped, the receiver is no longer connected to the bus:
    ///
    /// ```rust
    /// # #[cfg(feature = "unstable")]{
    /// use std::thread;
    /// use std::sync::mpsc;
    /// use maviola::sync::utils::mpmc;
    ///
    /// let (tx, rx) = mpmc::channel();
    /// let (rx_inner, guard) = rx.into_inner();
    ///
    /// let handler = { thread::spawn(move || -> Result<(), mpsc::RecvError> { rx_inner.recv() }) };
    ///
    /// drop(guard);
    ///
    /// assert!(tx.send(()).is_err());
    /// assert!(handler.join().unwrap().is_err());
    /// # }
    /// ```
    #[must_use]
    pub fn into_inner(self) -> (mpsc::Receiver<T>, RecvGuard<T>) {
        (self.inner, self.guard)
    }
}

impl<T: Clone + Sync + Send + 'static> Clone for Receiver<T> {
    fn clone(&self) -> Self {
        let (id, rx) = self.guard.bus.add();

        Receiver {
            inner: rx,
            guard: RecvGuard {
                id,
                bus: self.guard.bus.clone(),
            },
        }
    }
}

/// <sup>`⍚` | [`sync`](crate::sync)</sup>
/// Guards connection for the [`Receiver`].
///
/// When receiver is obtained from an inner [`mpsc::Receiver`] by calling [`Receiver::into_inner`],
/// then an instance of [`RecvGuard`] is returned. Once guard is dropped, the corresponding receiver
/// will be disconnected from the bus.
///
/// See [`Receiver::into_inner`] for details.
pub struct RecvGuard<T: Clone + Sync + Send + 'static> {
    id: UniqueId,
    bus: Arc<BroadcastBus<T>>,
}

impl<T: Clone + Sync + Send + 'static> Debug for RecvGuard<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RecvGuard").finish_non_exhaustive()
    }
}

impl<T: Clone + Sync + Send + 'static> Drop for RecvGuard<T> {
    fn drop(&mut self) {
        self.bus.remove(&self.id);
    }
}

struct BroadcastBus<T> {
    recv_txs: Arc<RwLock<HashMap<UniqueId, mpsc::Sender<T>>>>,
    _state: Closer,
}

impl<T: Clone + Sync + Send + 'static> BroadcastBus<T> {
    fn start(&self, send_rx: mpsc::Receiver<T>) {
        let recv_txs = self.recv_txs.clone();

        thread::spawn(move || loop {
            if recv_txs.read().unwrap().is_empty() {
                return;
            }

            let data = match send_rx.recv() {
                Ok(data) => data,
                Err(_) => {
                    let mut recv_txs = recv_txs.write().unwrap();
                    recv_txs.clear();
                    return;
                }
            };

            let failed_recv_tx_ids = {
                let recv_txs = recv_txs.read().unwrap();
                if recv_txs.is_empty() {
                    return;
                }

                let mut failed_recv_tx_ids = Vec::new();

                for (id, recv_tx) in recv_txs.iter() {
                    let recv_tx = recv_tx.clone();
                    let data = data.clone();
                    if recv_tx.send(data).is_err() {
                        failed_recv_tx_ids.push(*id);
                    }
                }

                failed_recv_tx_ids
            };

            if !failed_recv_tx_ids.is_empty() {
                let mut recv_txs = recv_txs.write().unwrap();
                for id in failed_recv_tx_ids {
                    recv_txs.remove(&id);
                }
            }
        });
    }

    fn add(&self) -> (UniqueId, mpsc::Receiver<T>) {
        let (recv_tx, recv_rx) = mpsc::channel();
        let id = UniqueId::new();

        let mut recv_txs = self.recv_txs.write().unwrap();
        recv_txs.insert(id, recv_tx);

        (id, recv_rx)
    }

    fn remove(&self, id: &UniqueId) {
        let mut recv_txs = self.recv_txs.write().unwrap();
        recv_txs.remove(id);
    }
}

impl<T> Drop for BroadcastBus<T> {
    fn drop(&mut self) {
        let mut recv_txs = self.recv_txs.write().unwrap();
        recv_txs.clear();
    }
}

/// <sup>`⍚` | [`sync`](crate::sync)</sup>
/// Creates a new asynchronous channel, returning the sender/receiver halves.
///
/// All data sent on the [`Sender`] will become available on the [`Receiver`] in
/// the same order as it was sent, and no [`Sender::send`] will block the calling thread.
/// [`Receiver::recv`] will block until a message is available while there is at least one
/// [Sender`] alive (including clones).
///
/// Behaves almost identical to [`mpsc::channel`] except that it supports multiple receivers to
/// which data will be broadcast.
#[must_use]
pub fn channel<T: Clone + Sync + Send + 'static>() -> (Sender<T>, Receiver<T>) {
    let (send_tx, send_rx) = mpsc::channel();
    let state = Closer::new();

    let sender = Sender {
        inner: send_tx,
        state: state.to_closable(),
    };

    let receiver = {
        let bus = BroadcastBus {
            recv_txs: Default::default(),
            _state: state,
        };

        let (id, rx) = bus.add();
        let bus = Arc::new(bus);

        let receiver = Receiver {
            inner: rx,
            guard: RecvGuard {
                id,
                bus: bus.clone(),
            },
        };

        bus.start(send_rx);

        receiver
    };

    (sender, receiver)
}

#[cfg(test)]
mod mpmc_test {
    use super::*;

    #[test]
    fn mpmc_basic() {
        let (tx, rx) = channel();

        let rx_1 = rx;
        let rx_2 = rx_1.clone();

        tx.send(1).unwrap();

        rx_1.recv().unwrap();
        rx_2.recv().unwrap();
    }

    #[test]
    fn mpmc_close_on_sender_dropped() {
        let (tx, rx) = channel();
        drop(tx);

        let res: Result<usize, _> = rx.recv();
        assert!(res.is_err());
    }

    #[test]
    fn mpmc_close_on_receivers_dropped() {
        let (tx, rx) = channel();
        drop(rx);

        assert!(tx.send(1).is_err());

        let (tx, rx) = channel();
        let rx_1 = rx;
        let rx_2 = rx_1.clone();
        drop(rx_1);
        drop(rx_2);

        assert!(tx.send(1).is_err());
    }

    #[test]
    fn mpmc_multi_threading() {
        let (tx, rx) = channel();

        let handler = thread::spawn(move || -> usize { rx.recv().unwrap() });

        tx.send(1).unwrap();

        let res = handler.join().unwrap();
        assert_eq!(res, 1);
    }

    #[test]
    fn inner_receiver_is_active_while_guard_is_present() {
        let (tx, rx) = channel();
        let (rx_inner, _guard) = rx.into_inner();

        let handler = thread::spawn(move || -> Result<(), mpsc::RecvError> { rx_inner.recv() });

        assert!(tx.send(()).is_ok());
        assert!(handler.join().unwrap().is_ok());
    }

    #[test]
    fn inner_receiver_disconnects_when_guard_is_dropped() {
        let (tx, rx) = channel();
        let (rx_inner, guard) = rx.into_inner();

        let handler = { thread::spawn(move || -> Result<(), mpsc::RecvError> { rx_inner.recv() }) };

        drop(guard);

        assert!(tx.send(()).is_err());
        assert!(handler.join().unwrap().is_err());
    }
}
