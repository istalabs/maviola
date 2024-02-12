//! Synchronous TCP connection.

use mavio::protocol::MaybeVersioned;
use std::net::TcpStream;
use std::sync::atomic::AtomicBool;
use std::sync::{atomic, mpsc, Arc, Mutex};

use crate::errors::NodeError;

use crate::prelude::*;

use crate::io::sync::connection::{Connection, ConnectionEvent, ConnectionInfo, Receiver, Sender};
use crate::protocol::CoreFrame;

/// TCP connection.
#[derive(Debug)]
pub struct TcpConnection<V: MaybeVersioned> {
    pub(super) id: usize,
    pub(super) info: ConnectionInfo,
    pub(super) receiver: Arc<Mutex<Box<dyn Receiver<V>>>>,
    pub(super) sender: Arc<Mutex<Box<dyn Sender<V>>>>,
    pub(super) events_chan: mpsc::Sender<ConnectionEvent<V>>,
}

impl<V: MaybeVersioned> Connection<V> for TcpConnection<V> {
    #[inline]
    fn id(&self) -> usize {
        self.id
    }

    #[inline]
    fn info(&self) -> &ConnectionInfo {
        &self.info
    }

    fn receiver(&self) -> Arc<Mutex<Box<dyn Receiver<V>>>> {
        self.receiver.clone()
    }

    fn sender(&self) -> Arc<Mutex<Box<dyn Sender<V>>>> {
        self.sender.clone()
    }

    fn close(&self) -> Result<()> {
        self.events_chan
            .send(ConnectionEvent::Drop(self.id, None))
            .map_err(Error::from)
    }
}

/// TCP [`Frame`] receiver.
#[derive(Debug)]
pub struct TcpReceiver<V: MaybeVersioned> {
    id: usize,
    conn_info: ConnectionInfo,
    conn_events_chan: mpsc::Sender<ConnectionEvent<V>>,
    receiver: mavio::Receiver<TcpStream, V>,
    is_active: AtomicBool,
}

impl<V: MaybeVersioned> TcpReceiver<V> {
    pub(crate) fn new(
        id: usize,
        conn_info: ConnectionInfo,
        conn_events_chan: mpsc::Sender<ConnectionEvent<V>>,
        receiver: mavio::Receiver<TcpStream, V>,
    ) -> Self {
        Self {
            id,
            conn_info,
            conn_events_chan,
            receiver,
            is_active: AtomicBool::new(true),
        }
    }
}

impl<V: MaybeVersioned> Receiver<V> for TcpReceiver<V> {
    fn recv(&mut self) -> Result<CoreFrame<V>> {
        if !self.is_active.load(atomic::Ordering::Relaxed) {
            return Err(NodeError::Inactive.into());
        }

        match self.receiver.recv() {
            Ok(res) => Ok(res),
            Err(err) => match err {
                mavio::errors::Error::Io(_) => {
                    self.is_active.store(false, atomic::Ordering::Relaxed);

                    let err = Error::from(err);
                    if let Err(err) = self
                        .conn_events_chan
                        .send(ConnectionEvent::Drop(self.id, Some(err.clone())))
                    {
                        log::error!(
                            "{:?}: can't pass receiver connection drop event: {err:?}",
                            self.conn_info
                        );
                    };

                    Err(err)
                }
                _ => Err(Error::from(err)),
            },
        }
    }
}

/// TCP [`Frame`] sender.
#[derive(Debug)]
pub struct TcpSender<V: MaybeVersioned> {
    id: usize,
    conn_info: ConnectionInfo,
    conn_events_chan: mpsc::Sender<ConnectionEvent<V>>,
    sender: mavio::Sender<TcpStream, V>,
    is_active: AtomicBool,
}

impl<V: MaybeVersioned> TcpSender<V> {
    pub(crate) fn new(
        id: usize,
        conn_info: ConnectionInfo,
        conn_events_chan: mpsc::Sender<ConnectionEvent<V>>,
        sender: mavio::Sender<TcpStream, V>,
    ) -> Self {
        Self {
            id,
            conn_info,
            conn_events_chan,
            sender,
            is_active: AtomicBool::new(true),
        }
    }
}

impl<V: MaybeVersioned> Sender<V> for TcpSender<V> {
    fn send(&mut self, frame: &CoreFrame<V>) -> Result<usize> {
        if !self.is_active.load(atomic::Ordering::Relaxed) {
            return Err(NodeError::Inactive.into());
        }

        match self.sender.send(frame) {
            Ok(res) => Ok(res),
            Err(err) => match err {
                mavio::errors::Error::Io(_) => {
                    self.is_active.store(false, atomic::Ordering::Relaxed);

                    let err = Error::from(err);
                    if let Err(err) = self
                        .conn_events_chan
                        .send(ConnectionEvent::Drop(self.id, Some(err.clone())))
                    {
                        log::error!(
                            "{:?}: can't pass sender connection drop event: {err:?}",
                            self.conn_info
                        );
                    };

                    Err(err)
                }
                _ => Err(Error::from(err)),
            },
        }
    }
}
