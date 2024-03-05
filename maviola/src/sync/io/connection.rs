use std::fmt::Debug;
use std::sync::mpsc;
use std::thread;
use std::thread::JoinHandle;

use crate::core::io::{ConnectionConf, ConnectionInfo, OutgoingFrame};
use crate::core::utils::{Closable, SharedCloser};
use crate::sync::consts::CONN_STOP_POOLING_INTERVAL;
use crate::sync::io::{Callback, ChannelFactory, FrameReceiver, FrameSender};

use crate::prelude::*;

/// <sup>[`sync`](crate::sync)</sup>
/// Connection builder used to create a [`Connection`].
pub trait ConnectionBuilder<V: MaybeVersioned>: ConnectionConf {
    /// Builds connection from provided configuration.
    ///
    /// Returns the new connection and its main handler. Once handler is finished, the connection
    /// is considered to be closed.
    fn build(&self) -> Result<(Connection<V>, ConnectionHandler)>;
}

/// <sup>[`sync`](crate::sync)</sup>
/// MAVLink connection.
#[derive(Debug)]
pub struct Connection<V: MaybeVersioned + 'static> {
    info: ConnectionInfo,
    sender: ConnSender<V>,
    receiver: ConnReceiver<V>,
    state: SharedCloser,
}

/// <sup>[`sync`](crate::sync)</sup>
/// Connection handler.
pub struct ConnectionHandler {
    inner: JoinHandle<Result<()>>,
}

impl ConnectionHandler {
    /// Spawns a new connection handler from a closure.
    pub fn spawn<F>(func: F) -> Self
    where
        F: FnOnce() -> Result<()>,
        F: Send + 'static,
    {
        Self {
            inner: thread::spawn(func),
        }
    }

    /// Spawns a new connection handler that finishes when the `state` becomes closed.
    pub fn spawn_from_state(state: SharedCloser) -> Self {
        Self::spawn(move || {
            while !state.is_closed() {
                thread::sleep(CONN_STOP_POOLING_INTERVAL);
            }
            Ok(())
        })
    }

    pub(crate) fn handle<V: MaybeVersioned>(self, conn: &Connection<V>) {
        let mut state = conn.state.clone();
        let info = conn.info.clone();

        thread::spawn(move || {
            while !self.inner.is_finished() {
                thread::sleep(CONN_STOP_POOLING_INTERVAL);
            }
            state.close();

            match self.inner.join() {
                Ok(res) => match res {
                    Ok(_) => log::debug!("[{info:?}] connection stopped"),
                    Err(err) => log::debug!("[{info:?}] connection exited with error: {err:?}"),
                },
                Err(err) => log::error!("[{info:?}] connection failed: {err:?}"),
            }
        });
    }
}

impl<V: MaybeVersioned> Connection<V> {
    /// Creates a new connection and associated [`ChannelFactory`].
    pub fn new(info: ConnectionInfo, state: SharedCloser) -> (Self, ChannelFactory<V>) {
        let (sender, send_handler) = mpmc::channel();
        let (producer, receiver) = mpmc::channel();

        let connection = Self {
            info,
            sender: ConnSender {
                state: state.to_closable(),
                sender: sender.clone(),
            },
            receiver: ConnReceiver { receiver },
            state,
        };

        let builder = ChannelFactory {
            info: connection.info.clone(),
            state: connection.state.to_closable(),
            sender,
            send_handler,
            producer,
        };

        (connection, builder)
    }

    /// Information about this connection.
    pub fn info(&self) -> &ConnectionInfo {
        &self.info
    }

    /// Send frame.
    #[inline]
    pub fn send(&self, frame: &Frame<V>) -> Result<()> {
        self.sender.send(frame)
    }

    /// Receive frame.
    ///
    /// Blocks until frame received.
    #[inline]
    pub fn recv(&self) -> Result<(Frame<V>, Callback<V>)> {
        self.receiver.recv()
    }

    /// Attempts to receive MAVLink frame without blocking.
    #[inline]
    pub fn try_recv(&self) -> Result<(Frame<V>, Callback<V>)> {
        self.receiver.try_recv()
    }

    pub(crate) fn share_state(&self) -> SharedCloser {
        self.state.clone()
    }

    pub(crate) fn sender(&self) -> ConnSender<V> {
        self.sender.clone()
    }

    pub(crate) fn receiver(&self) -> ConnReceiver<V> {
        self.receiver.clone()
    }

    fn close(&mut self) {
        if self.state.is_closed() {
            return;
        }
        self.state.close();
        log::debug!("[{:?}] connection closed", self.info);
    }
}

impl<V: MaybeVersioned> Drop for Connection<V> {
    fn drop(&mut self) {
        self.close();
    }
}

///////////////////////////////////////////////////////////////////////////////
//                                 PRIVATE                                   //
///////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Debug)]
pub(crate) struct ConnSender<V: MaybeVersioned + 'static> {
    sender: FrameSender<V>,
    state: Closable,
}

impl<V: MaybeVersioned> ConnSender<V> {
    pub(crate) fn send(&self, frame: &Frame<V>) -> Result<()> {
        if self.state.is_closed() {
            return Err(Error::from(mpsc::SendError(frame)));
        }

        self.sender
            .send(OutgoingFrame::new(frame.clone()))
            .map_err(Error::from)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ConnReceiver<V: MaybeVersioned + 'static> {
    receiver: FrameReceiver<V>,
}

impl<V: MaybeVersioned> ConnReceiver<V> {
    pub(crate) fn recv(&self) -> Result<(Frame<V>, Callback<V>)> {
        self.receiver.recv().map_err(Error::from)
    }

    pub(crate) fn try_recv(&self) -> Result<(Frame<V>, Callback<V>)> {
        self.receiver.try_recv().map_err(Error::from)
    }
}
