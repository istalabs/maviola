use std::sync::Arc;

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::broadcast::error::RecvError;

use crate::asnc::consts::{
    CHANNEL_STOP_JOIN_ATTEMPTS, CHANNEL_STOP_JOIN_POOLING_INTERVAL, CHANNEL_STOP_POOLING_INTERVAL,
};
use crate::asnc::io::{Callback, FrameProducer, FrameSendHandler, FrameSender};
use crate::core::io::{AsyncReceiver, AsyncSender, OutgoingFrame};
use crate::core::io::{ChannelInfo, ConnectionInfo};
use crate::core::utils::{Closable, SharedCloser, UniqueId};

use crate::prelude::*;

/// <sup>[`async`](crate::asnc)</sup>
/// Factory that produces a channels withing associated [`AsyncConnection`](super::Connection).
#[derive(Debug)]
pub struct ChannelFactory<V: MaybeVersioned + 'static> {
    pub(crate) info: ConnectionInfo,
    pub(crate) state: Closable,
    pub(crate) sender: FrameSender<V>,
    pub(crate) send_handler: FrameSendHandler<V>,
    pub(crate) producer: FrameProducer<V>,
}

impl<V: MaybeVersioned> ChannelFactory<V> {
    /// Builds a channel associated with the corresponding.
    #[must_use]
    pub fn build<R: AsyncRead + Unpin, W: AsyncWrite + Unpin>(
        &self,
        info: ChannelInfo,
        reader: R,
        writer: W,
    ) -> Channel<V, R, W> {
        Channel {
            conn_state: self.state.clone(),
            info,
            reader,
            writer,
            sender: self.sender.clone(),
            send_handler: self.send_handler.resubscribe(),
            producer: self.producer.clone(),
        }
    }

    /// Information about associated connection.
    pub fn info(&self) -> &ConnectionInfo {
        &self.info
    }

    /// Returns `true` if associated connection is already closed.
    pub fn is_closed(&self) -> bool {
        self.state.is_closed()
    }
}

/// <sup>[`async`](crate::asnc)</sup>
/// AsyncChannel associated with a particular connection.
///
/// Channels are constructed by [`ChannelFactory`] bound to a particular
/// [`AsyncConnection`](super::Connection).
pub struct Channel<V: MaybeVersioned + 'static, R: AsyncRead, W: AsyncWrite> {
    conn_state: Closable,
    info: ChannelInfo,
    reader: R,
    writer: W,
    sender: FrameSender<V>,
    send_handler: FrameSendHandler<V>,
    producer: FrameProducer<V>,
}

impl<
        V: MaybeVersioned + 'static,
        R: AsyncRead + Send + Unpin + 'static,
        W: AsyncWrite + Send + Unpin + 'static,
    > Channel<V, R, W>
{
    /// Spawn this channel.
    ///
    /// Returns [`SharedCloser`] which can be used to control channel state. The state of the
    /// channel depends on the state of the associated [`AsyncConnection`](super::Connection). However,
    /// it is not guaranteed, that channel will immediately close once connection is closed. There
    /// could be a lag relating to blocking nature of the underlying reader and writer.
    ///
    /// If caller is not interested in managing this channel, then it is required to drop returned
    /// [`SharedCloser`] or replace it with the corresponding [`Closable`].
    pub async fn spawn(self) -> SharedCloser {
        let id = UniqueId::new();
        let info = Arc::new(self.info);
        let conn_state = self.conn_state.clone();
        let state = SharedCloser::new();

        log::trace!("[{info:?}] spawning connection channel");

        let write_handler = {
            let send_handler = self.send_handler;
            let frame_writer = AsyncSender::new(self.writer);

            tokio::spawn(async move { Self::write_handler(id, send_handler, frame_writer).await })
        };

        let read_handler = {
            let conn_state = conn_state.clone();
            let state = state.clone();
            let info = info.clone();
            let sender = self.sender;
            let producer = self.producer;
            let frame_reader = AsyncReceiver::new(self.reader);

            tokio::spawn(async move {
                Self::read_handler(state, conn_state, id, info, sender, producer, frame_reader)
                    .await
            })
        };

        {
            let info = info.clone();
            let state = state.clone();
            tokio::spawn(async move {
                Self::handle_stop(state, conn_state, info, write_handler, read_handler).await;
            });
        }

        state.clone()
    }

    async fn write_handler(
        id: UniqueId,
        mut send_handler: FrameSendHandler<V>,
        mut frame_writer: AsyncSender<W, V>,
    ) -> Result<()> {
        loop {
            let out_frame = match send_handler.recv().await {
                Ok(out_frame) => out_frame,
                Err(err) => {
                    frame_writer.flush().await.map_err(Error::from)?;
                    return Err(Error::from(err));
                }
            };

            if !out_frame.should_send_to(id) {
                continue;
            }

            if let Err(err) = frame_writer.send(out_frame.frame()).await {
                let err = Error::from(err);
                if let Error::Io(err) = err {
                    if let std::io::ErrorKind::TimedOut = err.kind() {
                        continue;
                    }
                    return Err(Error::Io(err));
                }
            }
        }
    }

    async fn read_handler(
        state: SharedCloser,
        conn_state: Closable,
        id: UniqueId,
        info: Arc<ChannelInfo>,
        sender: FrameSender<V>,
        producer: FrameProducer<V>,
        mut frame_reader: AsyncReceiver<R, V>,
    ) -> Result<()> {
        loop {
            if conn_state.is_closed() || state.is_closed() {
                return Ok(());
            }

            let frame = match frame_reader.recv().await {
                Ok(frame) => frame,
                Err(err) => {
                    let err = Error::from(err);
                    if let Error::Io(err) = err {
                        if let std::io::ErrorKind::TimedOut = err.kind() {
                            continue;
                        }
                        return Err(Error::Io(err));
                    }
                    continue;
                }
            };

            let info = info.clone();
            let send_tx = sender.clone();

            let response = Callback {
                sender_id: id,
                sender_info: info.clone(),
                broadcast_tx: send_tx,
            };

            producer.send((frame, response))?;
        }
    }

    async fn handle_stop(
        mut state: SharedCloser,
        conn_state: Closable,
        info: Arc<ChannelInfo>,
        write_handler: tokio::task::JoinHandle<Result<()>>,
        read_handler: tokio::task::JoinHandle<Result<()>>,
    ) {
        while !(state.is_closed()
            || conn_state.is_closed()
            || write_handler.is_finished()
            || read_handler.is_finished())
        {
            tokio::time::sleep(CHANNEL_STOP_POOLING_INTERVAL).await;
        }
        state.close();

        for i in 0..CHANNEL_STOP_JOIN_ATTEMPTS {
            if write_handler.is_finished() && read_handler.is_finished() {
                break;
            }
            tokio::time::sleep(CHANNEL_STOP_JOIN_POOLING_INTERVAL).await;
            if i == CHANNEL_STOP_JOIN_ATTEMPTS - 1 {
                log::warn!(
                    "[{info:?}] write/read handlers are stuck, finished: write={}, read={}",
                    write_handler.is_finished(),
                    read_handler.is_finished()
                );
                return;
            }
        }

        if let (Ok(res_write), Ok(res_read)) = (write_handler.await, read_handler.await) {
            if let Err(err) = res_write {
                log::debug!("[{info:?}] write handler finished with error: {err:?}")
            }
            if let Err(err) = res_read {
                log::debug!("[{info:?}] read handler finished with error: {err:?}")
            }
        } else {
            log::error!("[{info:?}] error joining read/write handlers");
        }
        log::trace!("[{info:?}] handlers stopped");
    }
}
