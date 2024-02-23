use std::fmt::Debug;
use std::sync::mpsc;

use async_trait::async_trait;

use crate::asnc::conn::{AsyncChannelFactory, AsyncFrameReceiver, AsyncFrameSender};
use crate::asnc::consts::CONN_BROADCAST_CHAN_CAPACITY;
use crate::asnc::AsyncCallback;
use crate::core::io::{ConnectionInfo, OutgoingFrame};
use crate::core::utils::{Closable, SharedCloser};

use crate::prelude::*;

/// AsyncConnection builder used to create a [`AsyncConnection`].
#[async_trait]
pub trait AsyncConnectionBuilder<V: MaybeVersioned + 'static>: Debug + Send {
    /// Provides information about connection.
    fn info(&self) -> &ConnectionInfo;

    /// Builds [`AsyncConnection`] from provided configuration.
    async fn build(&self) -> Result<AsyncConnection<V>>;
}

/// MAVLink connection.
#[derive(Debug)]
pub struct AsyncConnection<V: MaybeVersioned + 'static> {
    info: ConnectionInfo,
    sender: AsyncConnSender<V>,
    receiver: AsyncConnReceiver<V>,
    state: SharedCloser,
}

impl<V: MaybeVersioned> AsyncConnection<V> {
    /// Creates a new connection and associated [`AsyncChannelFactory`].
    pub fn new(info: ConnectionInfo, state: SharedCloser) -> (Self, AsyncChannelFactory<V>) {
        let (sender, send_handler) = broadcast::channel(CONN_BROADCAST_CHAN_CAPACITY);
        let (producer, receiver) = broadcast::channel(CONN_BROADCAST_CHAN_CAPACITY);

        let connection = Self {
            info,
            sender: AsyncConnSender {
                state: state.to_closable(),
                sender: sender.clone(),
            },
            receiver: AsyncConnReceiver { receiver },
            state,
        };

        let builder = AsyncChannelFactory {
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
    pub fn send(&self, frame: &Frame<V>) -> Result<usize> {
        self.sender.send(frame)
    }

    /// Receive frame.
    ///
    /// Await until frame received.
    #[inline]
    pub async fn recv(&mut self) -> Result<(Frame<V>, AsyncCallback<V>)> {
        self.receiver.recv().await
    }

    /// Attempts to receive MAVLink frame without blocking.
    #[inline]
    pub fn try_recv(&mut self) -> Result<(Frame<V>, AsyncCallback<V>)> {
        self.receiver.try_recv()
    }

    /// Close connection.
    pub fn close(&mut self) {
        if self.state.is_closed() {
            return;
        }
        self.state.close();
        log::debug!("[{:?}] connection closed", self.info);
    }

    pub(crate) fn sender(&self) -> AsyncConnSender<V> {
        self.sender.clone()
    }

    pub(crate) fn receiver(&self) -> AsyncConnReceiver<V> {
        self.receiver.clone()
    }
}

impl<V: MaybeVersioned> Drop for AsyncConnection<V> {
    fn drop(&mut self) {
        self.close();
    }
}

///////////////////////////////////////////////////////////////////////////////
//                                 PRIVATE                                   //
///////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Debug)]
pub(crate) struct AsyncConnSender<V: MaybeVersioned + 'static> {
    sender: AsyncFrameSender<V>,
    state: Closable,
}

impl<V: MaybeVersioned> AsyncConnSender<V> {
    pub(crate) fn send(&self, frame: &Frame<V>) -> Result<usize> {
        if self.state.is_closed() {
            return Err(Error::from(mpsc::SendError(frame)));
        }

        self.sender
            .send(OutgoingFrame::new(frame.clone()))
            .map_err(Error::from)
    }
}

#[derive(Debug)]
pub(crate) struct AsyncConnReceiver<V: MaybeVersioned + 'static> {
    receiver: AsyncFrameReceiver<V>,
}

impl<V: MaybeVersioned + 'static> Clone for AsyncConnReceiver<V> {
    fn clone(&self) -> Self {
        Self {
            receiver: self.receiver.resubscribe(),
        }
    }
}

impl<V: MaybeVersioned> AsyncConnReceiver<V> {
    pub(crate) async fn recv(&mut self) -> Result<(Frame<V>, AsyncCallback<V>)> {
        self.receiver.recv().await.map_err(Error::from)
    }

    pub(crate) fn try_recv(&mut self) -> Result<(Frame<V>, AsyncCallback<V>)> {
        self.receiver.try_recv().map_err(Error::from)
    }
}
