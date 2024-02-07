//! Synchronous TCP connection.

use std::net::TcpStream;
use std::sync::{mpsc, Arc, Mutex};

use mavio::Frame;

use crate::errors::{Error, Result};
use crate::io::sync::connection::{Connection, ConnectionEvent, ConnectionInfo, Receiver, Sender};

/// TCP connection.
#[derive(Debug)]
pub struct TcpConnection {
    pub(super) id: usize,
    pub(super) info: ConnectionInfo,
    pub(super) receiver: Arc<Mutex<Box<dyn Receiver>>>,
    pub(super) sender: Arc<Mutex<Box<dyn Sender>>>,
    pub(super) event_chan: mpsc::Sender<ConnectionEvent>,
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
        self.event_chan
            .send(ConnectionEvent::Drop(self.id, None))
            .map_err(Error::from)
    }
}

/// TCP [`Frame`] receiver.
#[derive(Debug)]
pub struct TcpReceiver {
    pub(super) id: usize,
    pub(super) event_chan: mpsc::Sender<ConnectionEvent>,
    pub(crate) receiver: mavio::Receiver<TcpStream>,
}

impl Receiver for TcpReceiver {
    fn recv(&mut self) -> Result<Frame> {
        match self.receiver.recv() {
            Ok(res) => Ok(res),
            Err(err) => match err {
                mavio::errors::CoreError::Io(_) => {
                    let err = Error::from(err);
                    if let Err(err) = self
                        .event_chan
                        .send(ConnectionEvent::Drop(self.id, Some(err.clone())))
                    {
                        log::error!("can't pass connection drop error: {err:?}");
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
    pub(super) id: usize,
    pub(super) event_chan: mpsc::Sender<ConnectionEvent>,
    pub(crate) sender: mavio::Sender<TcpStream>,
}

impl Sender for TcpSender {
    fn send(&mut self, frame: &Frame) -> Result<usize> {
        match self.sender.send(frame) {
            Ok(res) => Ok(res),
            Err(err) => match err {
                mavio::errors::CoreError::Io(_) => {
                    let err = Error::from(err);
                    if let Err(err) = self
                        .event_chan
                        .send(ConnectionEvent::Drop(self.id, Some(err.clone())))
                    {
                        log::error!("can't pass connection drop error: {err:?}");
                    };
                    Err(err)
                }
                _ => Err(Error::from(err)),
            },
        }
    }
}
