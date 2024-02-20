use std::fmt::Debug;
use std::sync::atomic::AtomicBool;
use std::sync::{atomic, mpsc, Arc};

use tokio::io::{AsyncRead, AsyncWrite};

use mavio::protocol::MaybeVersioned;
use mavio::{AsyncReceiver, AsyncSender, Frame};

use crate::io::asnc::response::AsyncResponse;
use crate::io::broadcast::OutgoingFrame;
use crate::io::{ConnectionInfo, PeerConnectionInfo};
use crate::utils::UniqueId;

use crate::prelude::*;

/// Connection configuration.
pub trait AsyncConnectionConf<V: MaybeVersioned>: AsyncConnectionBuilder<V> {
    fn info(&self) -> &ConnectionInfo;
}

/// Connection builder used to create a [`AsyncConnection`].
pub trait AsyncConnectionBuilder<V: MaybeVersioned>: Debug + Send {
    /// Builds [`AsyncConnection`] from provided configuration.
    fn build(&self) -> Result<AsyncConnection<V>>;
}

/// MAVLink connection.
#[derive(Debug)]
pub struct AsyncConnection<V: MaybeVersioned + 'static> {
    info: ConnectionInfo,
    sender: AsyncFrameSender<V>,
    receiver: AsyncFrameReceiver<V>,
    is_active: Arc<AtomicBool>,
}

impl<V: MaybeVersioned> AsyncConnection<V> {
    pub(super) fn new(
        info: ConnectionInfo,
        sender: AsyncFrameSender<V>,
        receiver: AsyncFrameReceiver<V>,
    ) -> Self {
        Self {
            info,
            sender,
            receiver,
            is_active: Arc::new(AtomicBool::new(true)),
        }
    }

    /// Information about this connection.
    pub fn info(&self) -> &ConnectionInfo {
        &self.info
    }

    /// Send frame.
    pub fn send(&self, frame: &Frame<V>) -> Result<usize> {
        if !self.is_active.load(atomic::Ordering::Relaxed) {
            return Err(Error::from(mpsc::SendError(frame)));
        }

        self.sender
            .send(OutgoingFrame::new(frame.clone()))
            .map_err(Error::from)
    }

    /// Receive frame.
    ///
    /// Blocks until frame received.
    pub async fn recv(&mut self) -> Result<(Frame<V>, AsyncResponse<V>)> {
        if !self.is_active.load(atomic::Ordering::Relaxed) {
            return Err(Error::from(mpsc::RecvError));
        }

        self.receiver.recv().await.map_err(Error::from)
    }

    /// Attempts to receive MAVLink frame without blocking.
    pub fn try_recv(&mut self) -> Result<(Frame<V>, AsyncResponse<V>)> {
        if !self.is_active.load(atomic::Ordering::Relaxed) {
            return Err(Error::from(mpsc::TryRecvError::Disconnected));
        }

        self.receiver.try_recv().map_err(Error::from)
    }

    /// Close connection.
    pub fn close(&mut self) {
        self.is_active.store(false, atomic::Ordering::Relaxed);
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

pub(super) type AsyncFrameSender<V> = broadcast::Sender<OutgoingFrame<V>>;
pub(super) type AsyncFrameSendHandler<V> = broadcast::Receiver<OutgoingFrame<V>>;
pub(super) type AsyncFrameRecvDispatcher<V> = broadcast::Sender<(Frame<V>, AsyncResponse<V>)>;
pub(super) type AsyncFrameReceiver<V> = broadcast::Receiver<(Frame<V>, AsyncResponse<V>)>;

pub(super) struct AsyncPeerConnection<
    V: MaybeVersioned + 'static,
    R: AsyncRead,
    W: AsyncWrite + Unpin,
> {
    pub(super) info: PeerConnectionInfo,
    pub(super) reader: R,
    pub(super) writer: W,
    pub(super) send_tx: AsyncFrameSender<V>,
    pub(super) send_rx: AsyncFrameSendHandler<V>,
    pub(super) recv_tx: AsyncFrameRecvDispatcher<V>,
}

impl<
        V: MaybeVersioned + 'static,
        R: AsyncRead + Send + Unpin + 'static,
        W: AsyncWrite + Send + Unpin + 'static,
    > AsyncPeerConnection<V, R, W>
{
    pub(super) fn start(self) {
        let id = UniqueId::new();
        let info = Arc::new(self.info);
        let is_active = Arc::new(AtomicBool::new(true));

        {
            let is_active = is_active.clone();
            let info = info.clone();
            let send_rx = self.send_rx;
            let sender = AsyncSender::new(self.writer);

            tokio::spawn(async move {
                Self::send_handler(is_active, id, info, send_rx, sender).await;
            });
        }

        {
            let is_active = is_active.clone();
            let info = info.clone();
            let send_tx = self.send_tx;
            let recv_tx = self.recv_tx;
            let receiver = AsyncReceiver::new(self.reader);

            tokio::spawn(async move {
                Self::recv_handler(is_active, id, info, send_tx, recv_tx, receiver).await
            });
        }
    }

    async fn send_handler(
        is_active: Arc<AtomicBool>,
        id: UniqueId,
        info: Arc<PeerConnectionInfo>,
        mut send_rx: AsyncFrameSendHandler<V>,
        mut sender: AsyncSender<W, V>,
    ) {
        loop {
            if !is_active.load(atomic::Ordering::Relaxed) {
                log::trace!("[{info:?}] connection is inactive, stopping send handlers");
                return;
            }

            let out_frame = match send_rx.recv().await {
                Ok(frame) => frame,
                Err(err) => {
                    log::trace!("[{info:?}] can't receive outgoing frame: {err:?}");
                    is_active.store(false, atomic::Ordering::Relaxed);
                    return;
                }
            };

            if !out_frame.should_send_to(id) {
                continue;
            }

            if let Err(err) = sender.send(out_frame.frame()).await {
                log::trace!("[{info:?}] can't send a frame: {err:?}");

                let err = Error::from(err);
                if let Error::Io(err) = err {
                    log::trace!("[{info:?}] I/O error sending frame: {err:?}");
                    is_active.store(false, atomic::Ordering::Relaxed);
                    return;
                }
            }
        }
    }

    async fn recv_handler(
        is_active: Arc<AtomicBool>,
        id: UniqueId,
        info: Arc<PeerConnectionInfo>,
        send_tx: AsyncFrameSender<V>,
        recv_tx: AsyncFrameRecvDispatcher<V>,
        mut receiver: AsyncReceiver<R, V>,
    ) {
        loop {
            if !is_active.load(atomic::Ordering::Relaxed) {
                log::trace!("[{info:?}] connection is inactive, stopping send handlers");
                return;
            }

            let frame = match receiver.recv().await {
                Ok(frame) => frame,
                Err(err) => {
                    log::trace!("[{info:?}] can't receive incoming frame: {err:?}");

                    let err = Error::from(err);
                    if let Error::Io(err) = err {
                        log::trace!("[{info:?}] I/O error receiving frame: {err:?}");
                        is_active.store(false, atomic::Ordering::Relaxed);
                        return;
                    }
                    continue;
                }
            };

            let info = info.clone();
            let send_tx = send_tx.clone();

            let response = AsyncResponse {
                sender_id: id,
                sender_info: info.clone(),
                broadcast_tx: send_tx,
            };

            if let Err(err) = recv_tx.send((frame, response)) {
                log::trace!("[{info:?}] can't pass incoming frame: {err:?}");
                is_active.store(false, atomic::Ordering::Relaxed);
                return;
            }
        }
    }
}
