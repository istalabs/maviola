//! # Multiple producers / multiple consumers broadcast channel
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
//! [`Receiver`] can be cloned. A cloned receiver becomes an independent listener for channel
//! messages.
//!
//! In addition, both sender and receiver implement a `disconnect` method which severs connection to
//! the channel. Disconnected senders and receivers will return standard [`mpsc`] errors on
//! send / receive attempts.
//!
//! # Examples
//!
//! ```rust
//! # #[cfg(feature = "sync")]
//! # {
//! use maviola::io::sync::utils::mpmc;
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
//!
//! # Limitation
//!
//! Due to the current implementation, it is possible that message, which has been successfully
//! sent, won't be consumed if all consumers had been disconnected right before the sending. This
//! means that caller should not rely on the [`Ok`] result of the [`Sender::send`] in scenarios,
//! where message consumption must be acknowledged by any means. If this is the case, then it is
//! suggested to fall back to [`mpsc`] primitives.   

use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::sync::{mpsc, Arc, RwLock};
use std::{mem, thread};

use crate::utils::UniqueId;

/// MPMC sender.
///
/// Behaves almost identical to [`mpsc::Sender`] except it has a
/// [`disconnect`](Receiver::disconnect) method that disconnect sender from its channel.
#[derive(Clone, Debug)]
pub struct Sender<T> {
    inner: Option<mpsc::Sender<T>>,
}

unsafe impl<T: Send> Send for Sender<T> {}
unsafe impl<T: Sync> Sync for Sender<T> {}

impl<T> Sender<T> {
    /// Attempts to send a value on this channel, returning it back if it could
    /// not be sent.
    ///
    /// Behaves identical to [`mpsc::Sender::send`].
    pub fn send(&self, value: T) -> Result<(), mpsc::SendError<T>> {
        match &self.inner {
            None => Err(mpsc::SendError(value)),
            Some(inner) => inner.send(value),
        }
    }

    /// Disconnect sender from channel.
    pub fn disconnect(&mut self) {
        self.inner = None;
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        self.disconnect()
    }
}

/// MPMC receiver.
///
/// Behaves almost identical to [`mpsc::Receiver`] except it can be cloned and disconnected from the
/// channel by calling [`disconnect`](Receiver::disconnect) method.
///
/// Each cloned receiver will receive its own message.
pub struct Receiver<T: Clone + Sync + Send + 'static> {
    id: UniqueId,
    inner: Option<mpsc::Receiver<T>>,
    bus: Option<Arc<BroadcastBus<T>>>,
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
        match &self.inner {
            None => Err(mpsc::RecvError),
            Some(inner) => inner.recv(),
        }
    }

    /// Attempts to return a pending value on this receiver without blocking.
    ///
    /// Behaves identical to [`mpsc::Receiver::try_recv`].
    pub fn try_recv(&self) -> Result<T, mpsc::TryRecvError> {
        match &self.inner {
            None => Err(mpsc::TryRecvError::Disconnected),
            Some(inner) => inner.try_recv(),
        }
    }

    /// Disconnect receiver from the channel.
    pub fn disconnect(&mut self) {
        {
            let mut bus = None;
            mem::swap(&mut bus, &mut self.bus);
            if let Some(bus) = bus {
                bus.remove(&self.id);
            }
        }
        self.bus = None;
        self.inner = None;
    }
}
impl<T: Clone + Sync + Send + 'static> Clone for Receiver<T> {
    fn clone(&self) -> Self {
        match &self.bus {
            None => Receiver {
                id: UniqueId::new(),
                inner: None,
                bus: None,
            },
            Some(bus) => {
                let (id, rx) = bus.add();

                Receiver {
                    id,
                    inner: Some(rx),
                    bus: self.bus.clone(),
                }
            }
        }
    }
}

impl<T: Clone + Sync + Send + 'static> Drop for Receiver<T> {
    fn drop(&mut self) {
        self.disconnect();
    }
}

struct BroadcastBus<T> {
    recv_txs: Arc<RwLock<HashMap<UniqueId, mpsc::Sender<T>>>>,
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

    let sender = Sender {
        inner: Some(send_tx),
    };

    let receiver = {
        let bus = BroadcastBus {
            recv_txs: Default::default(),
        };

        let (id, rx) = bus.add();
        let bus = Arc::new(bus);

        let receiver = Receiver {
            id,
            inner: Some(rx),
            bus: Some(bus.clone()),
        };

        bus.start(send_rx);

        receiver
    };

    (sender, receiver)
}

#[cfg(test)]
mod mpmc_test {
    use super::*;
    use crate::utils::test::*;

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
        wait();

        assert!(tx.send(1).is_err());

        let (tx, rx) = channel();
        let rx_1 = rx;
        let rx_2 = rx_1.clone();
        drop(rx_1);
        drop(rx_2);
        wait_long();

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
}
