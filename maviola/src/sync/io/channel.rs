use std::io::{Read, Write};
use std::sync::Arc;
use std::thread;

use crate::core::io::{ChannelInfo, ConnectionInfo};
use crate::core::io::{Receiver, Sender};
use crate::core::utils::{Closable, SharedCloser, UniqueId};
use crate::sync::consts::{
    CHANNEL_STOP_JOIN_ATTEMPTS, CHANNEL_STOP_JOIN_POOLING_INTERVAL, CHANNEL_STOP_POOLING_INTERVAL,
};
use crate::sync::io::{Callback, IncomingFrameProducer, OutgoingFrameHandler, OutgoingFrameSender};

use crate::prelude::*;

/// <sup>[`sync`](crate::sync)</sup>
/// Factory that produces channels withing associated [`Connection`](super::Connection).
#[derive(Clone, Debug)]
pub struct ChannelFactory<V: MaybeVersioned + 'static> {
    pub(super) info: ConnectionInfo,
    pub(super) state: Closable,
    pub(super) sender: OutgoingFrameSender<V>,
    pub(super) send_handler: OutgoingFrameHandler<V>,
    pub(super) producer: IncomingFrameProducer<V>,
}

impl<V: MaybeVersioned> ChannelFactory<V> {
    /// Builds a channel associated with the corresponding.
    #[must_use]
    pub fn build<R: Read, W: Write>(
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
            send_handler: self.send_handler.clone(),
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

    /// Returns a producer of incoming frames.
    pub fn producer(&self) -> &IncomingFrameProducer<V> {
        &self.producer
    }

    /// Returns a sender for outgoing frames.
    pub fn sender(&self) -> &OutgoingFrameSender<V> {
        &self.sender
    }

    /// Returns a handler for outgoing frames.
    pub fn send_handler(&self) -> &OutgoingFrameHandler<V> {
        &self.send_handler
    }
}

/// <sup>[`sync`](crate::sync)</sup>
/// Channel associated with a particular connection.
///
/// Channels are constructed by [`ChannelFactory`] bound to a particular
/// [`Connection`](super::Connection).
pub struct Channel<V: MaybeVersioned + 'static, R: Read, W: Write> {
    conn_state: Closable,
    info: ChannelInfo,
    reader: R,
    writer: W,
    sender: OutgoingFrameSender<V>,
    send_handler: OutgoingFrameHandler<V>,
    producer: IncomingFrameProducer<V>,
}

impl<V: MaybeVersioned + 'static, R: Read + Send + 'static, W: Write + Send + 'static>
    Channel<V, R, W>
{
    /// Spawn this channel.
    ///
    /// Returns [`SharedCloser`] which can be used to control channel state. The state of the
    /// channel depends on the state of the associated [`Connection`](super::Connection). However,
    /// it is not guaranteed, that channel will immediately close once connection is closed. There
    /// could be a lag relating to blocking nature of the underlying reader and writer.
    ///
    /// If caller is not interested in managing this channel, then it is required to drop returned
    /// [`SharedCloser`] or replace it with the corresponding [`Closable`].
    pub fn spawn(self) -> SharedCloser {
        let id = UniqueId::new();
        let info = Arc::new(self.info);
        let conn_state = self.conn_state.clone();
        let state = SharedCloser::new();

        log::trace!("[{info:?}] spawning peer connection");

        let write_handler = {
            let info = info.clone();
            let send_handler = self.send_handler;
            let frame_writer = Sender::new(self.writer);

            thread::spawn(move || Self::write_handler(id, info, send_handler, frame_writer))
        };

        let read_handler = {
            let conn_state = conn_state.clone();
            let state = state.clone();
            let info = info.clone();
            let sender = self.sender;
            let producer = self.producer;
            let frame_reader = Receiver::new(self.reader);

            thread::spawn(move || {
                Self::read_handler(state, conn_state, id, info, sender, producer, frame_reader)
            })
        };

        {
            let info = info.clone();
            let state = state.clone();
            thread::spawn(move || {
                Self::handle_stop(state, conn_state, info, write_handler, read_handler);
            });
        }

        state.clone()
    }

    fn write_handler(
        id: UniqueId,
        info: Arc<ChannelInfo>,
        send_handler: OutgoingFrameHandler<V>,
        mut frame_writer: Sender<W, V>,
    ) -> Result<()> {
        loop {
            let out_frame = match send_handler.recv() {
                Ok(out_frame) => out_frame,
                Err(err) => {
                    frame_writer.flush().map_err(Error::from)?;
                    return Err(Error::from(err));
                }
            };

            if !out_frame.should_send_to(id) {
                continue;
            }

            log::trace!("[{info:?}] received outgoing frame from API");
            loop {
                if let Err(err) = frame_writer.send(out_frame.frame()) {
                    let err = Error::from(err);
                    if let Error::Io(err) = err {
                        if let std::io::ErrorKind::TimedOut = err.kind() {
                            continue;
                        }
                        return Err(Error::Io(err));
                    }
                }
                log::trace!("[{info:?}] written outgoing frame");
                break;
            }
        }
    }

    fn read_handler(
        state: SharedCloser,
        conn_state: Closable,
        id: UniqueId,
        info: Arc<ChannelInfo>,
        sender: OutgoingFrameSender<V>,
        producer: IncomingFrameProducer<V>,
        mut frame_reader: Receiver<R, V>,
    ) -> Result<()> {
        loop {
            if conn_state.is_closed() || state.is_closed() {
                return Ok(());
            }

            let frame = match frame_reader.recv() {
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
            log::trace!("[{info:?}] received incoming frame");

            let info = info.clone();
            let send_tx = sender.clone();

            let callback = Callback::new(id, info.clone(), send_tx);

            producer.send((frame, callback))?;
            log::trace!("[{info:?}] sent incoming frame to API");
        }
    }

    fn handle_stop(
        mut state: SharedCloser,
        conn_state: Closable,
        info: Arc<ChannelInfo>,
        write_handler: thread::JoinHandle<Result<()>>,
        read_handler: thread::JoinHandle<Result<()>>,
    ) {
        while !(state.is_closed()
            || conn_state.is_closed()
            || write_handler.is_finished()
            || read_handler.is_finished())
        {
            thread::sleep(CHANNEL_STOP_POOLING_INTERVAL);
        }
        state.close();

        for i in 0..CHANNEL_STOP_JOIN_ATTEMPTS {
            if write_handler.is_finished() && read_handler.is_finished() {
                break;
            }
            thread::sleep(CHANNEL_STOP_JOIN_POOLING_INTERVAL);
            if i == CHANNEL_STOP_JOIN_ATTEMPTS - 1 {
                log::warn!(
                    "[{info:?}] write/read handlers are stuck, finished: write={}, read={}",
                    write_handler.is_finished(),
                    read_handler.is_finished()
                );
                return;
            }
        }

        if let (Ok(res_write), Ok(res_read)) = (write_handler.join(), read_handler.join()) {
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
