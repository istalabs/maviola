//! Synchronous TCP connection.

use std::net::TcpStream;
use std::sync::atomic::AtomicBool;
use std::sync::{atomic, mpsc, Arc, Mutex};

use crate::errors::NodeError;

use crate::prelude::*;

use crate::io::sync::connection::{Connection, ConnectionEvent, ConnectionInfo, Receiver, Sender};
use crate::protocol::CoreFrame;

/// TCP connection.
#[derive(Debug)]
pub struct TcpConnection {
    pub(super) id: usize,
    pub(super) info: ConnectionInfo,
    pub(super) receiver: Arc<Mutex<Box<dyn Receiver>>>,
    pub(super) sender: Arc<Mutex<Box<dyn Sender>>>,
    pub(super) events_chan: mpsc::Sender<ConnectionEvent>,
}

impl Connection for TcpConnection {
    #[inline]
    fn id(&self) -> usize {
        self.id
    }

    #[inline]
    fn info(&self) -> &ConnectionInfo {
        &self.info
    }

    fn receiver(&self) -> Arc<Mutex<Box<dyn Receiver>>> {
        self.receiver.clone()
    }

    fn sender(&self) -> Arc<Mutex<Box<dyn Sender>>> {
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
pub struct TcpReceiver {
    id: usize,
    conn_info: ConnectionInfo,
    conn_events_chan: mpsc::Sender<ConnectionEvent>,
    receiver: mavio::Receiver<TcpStream>,
    is_active: AtomicBool,
}

impl TcpReceiver {
    pub(crate) fn new(
        id: usize,
        conn_info: ConnectionInfo,
        conn_events_chan: mpsc::Sender<ConnectionEvent>,
        receiver: mavio::Receiver<TcpStream>,
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

impl Receiver for TcpReceiver {
    fn recv(&mut self) -> Result<CoreFrame> {
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
pub struct TcpSender {
    id: usize,
    conn_info: ConnectionInfo,
    conn_events_chan: mpsc::Sender<ConnectionEvent>,
    sender: mavio::Sender<TcpStream>,
    is_active: AtomicBool,
}

impl TcpSender {
    pub(crate) fn new(
        id: usize,
        conn_info: ConnectionInfo,
        conn_events_chan: mpsc::Sender<ConnectionEvent>,
        sender: mavio::Sender<TcpStream>,
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

impl Sender for TcpSender {
    fn send(&mut self, frame: &CoreFrame) -> Result<usize> {
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
