use std::fmt::Debug;
use std::future::Future;

use async_trait::async_trait;
use tokio::task::JoinHandle;

use crate::asnc::consts::{CONN_BROADCAST_CHAN_CAPACITY, CONN_STOP_POOLING_INTERVAL};
use crate::asnc::io::{Callback, ChannelFactory, IncomingFrameReceiver, OutgoingFrameSender};
use crate::core::error::{RecvError, SendError, TryRecvError};
use crate::core::io::{ConnectionConf, ConnectionInfo, OutgoingFrame};
use crate::core::utils::{Closable, SharedCloser};

use crate::prelude::*;

/// <sup>[`async`](crate::asnc)</sup>
/// Connection builder used to create a [`Connection`].
#[async_trait]
pub trait ConnectionBuilder<V: MaybeVersioned + 'static>: ConnectionConf {
    /// Builds connection from provided configuration.
    ///
    /// Returns the new connection and its main handler. Once handler is finished, the connection
    /// is considered to be closed.
    async fn build(&self) -> Result<(Connection<V>, ConnectionHandler)>;
}

/// <sup>[`async`](crate::asnc)</sup>
/// Asynchronous MAVLink connection.
#[derive(Debug)]
pub struct Connection<V: MaybeVersioned + 'static> {
    info: ConnectionInfo,
    sender: ConnSender<V>,
    receiver: ConnReceiver<V>,
    state: SharedCloser,
}

/// <sup>[`async`](crate::asnc)</sup>
/// Connection handler.
pub struct ConnectionHandler {
    inner: JoinHandle<Result<()>>,
}

impl ConnectionHandler {
    /// Spawns a new connection handler from a future.
    pub fn spawn<F>(task: F) -> Self
    where
        F: Future<Output = Result<()>> + Send + 'static,
    {
        Self {
            inner: tokio::spawn(task),
        }
    }

    /// Spawns a new connection handler that finishes when the `state` becomes closed.
    pub fn spawn_from_state(state: SharedCloser) -> Self {
        Self::spawn(async move {
            while !state.is_closed() {
                tokio::time::sleep(CONN_STOP_POOLING_INTERVAL).await;
            }
            Ok(())
        })
    }

    pub(crate) fn handle<V: MaybeVersioned>(self, conn: &Connection<V>) {
        let mut state = conn.state.clone();
        let info = conn.info.clone();

        tokio::task::spawn(async move {
            let result = self.inner.await;
            state.close();

            match result {
                Ok(res) => match res {
                    Ok(_) => log::debug!("[{info:?}] listener stopped"),
                    Err(err) => log::debug!("[{info:?}] listener exited with error: {err:?}"),
                },
                Err(err) => log::error!("[{info:?}] listener failed: {err:?}"),
            }
        });
    }
}

impl<V: MaybeVersioned> Connection<V> {
    /// Creates a new connection and associated [`ChannelFactory`].
    pub fn new(info: ConnectionInfo, state: SharedCloser) -> (Self, ChannelFactory<V>) {
        let (sender, send_handler) = broadcast::channel(CONN_BROADCAST_CHAN_CAPACITY);
        let (producer, receiver) = broadcast::channel(CONN_BROADCAST_CHAN_CAPACITY);

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
    sender: OutgoingFrameSender<V>,
    state: Closable,
}

impl<V: MaybeVersioned> ConnSender<V> {
    pub(crate) fn send(&self, frame: Frame<V>) -> Result<()> {
        if self.state.is_closed() {
            return Err(Error::from(SendError(frame)));
        }

        self.sender
            .send(OutgoingFrame::new(frame))
            .map_err(Error::from)
            .map(|_| ())
    }
}

#[derive(Debug)]
pub(crate) struct ConnReceiver<V: MaybeVersioned + 'static> {
    receiver: IncomingFrameReceiver<V>,
}

impl<V: MaybeVersioned + 'static> Clone for ConnReceiver<V> {
    fn clone(&self) -> Self {
        Self {
            receiver: self.receiver.resubscribe(),
        }
    }
}

impl<V: MaybeVersioned> ConnReceiver<V> {
    pub(crate) async fn recv(
        &mut self,
    ) -> core::result::Result<(Frame<V>, Callback<V>), RecvError> {
        self.receiver.recv().await.map_err(RecvError::from)
    }

    pub(crate) fn try_recv(
        &mut self,
    ) -> core::result::Result<(Frame<V>, Callback<V>), TryRecvError> {
        self.receiver.try_recv().map_err(TryRecvError::from)
    }
}
