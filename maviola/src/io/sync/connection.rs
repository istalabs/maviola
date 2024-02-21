//! # Connection and channels
//!
//! This module contains abstractions for connections and channels.

use std::fmt::Debug;
use std::io::{Read, Write};
use std::sync::{mpsc, Arc};
use std::thread;

use mavio::protocol::MaybeVersioned;
use mavio::{Frame, Receiver, Sender};

use crate::io::broadcast::OutgoingFrame;
use crate::io::sync::callback::Callback;
use crate::io::sync::consts::{
    PEER_CONN_STOP_JOIN_ATTEMPTS, PEER_CONN_STOP_JOIN_POOLING_INTERVAL,
    PEER_CONN_STOP_POOLING_INTERVAL,
};
use crate::io::{ConnectionInfo, PeerConnectionInfo};
use crate::utils::{Closable, SharedCloser, UniqueId};

use crate::prelude::*;

/// Connection configuration.
pub trait ConnectionConf<V: MaybeVersioned>: ConnectionBuilder<V> {
    fn info(&self) -> &ConnectionInfo;
}

/// Connection builder used to create a [`Connection`].
pub trait ConnectionBuilder<V: MaybeVersioned>: Debug + Send {
    /// Builds [`Connection`] from provided configuration.
    fn build(&self) -> Result<Connection<V>>;
}

/// Sending part of channel that sends outgoing frames for [`Connection`].
///
/// Paired with [`FrameSendHandler`] that usually is owned by a [`Channel`].
pub type FrameSender<V> = mpmc::Sender<OutgoingFrame<V>>;
/// Receives incoming frames.
///
/// Paired with [`FrameProducer`] that usually owned by a [`Channel`] and receives incoming frames
/// from the underlying transport.
pub type FrameReceiver<V> = mpmc::Receiver<(Frame<V>, Callback<V>)>;
/// Handles outgoing frames produced by [`Connection::send`] or [`ConnSender::send`].
///
/// Usually owned by channels which intercept outgoing frames and write them to the underlying
/// transport. Paired with [`FrameSender`] which is owned by [`Connection`] and related
/// [`ConnSender`].
pub type FrameSendHandler<V> = mpmc::Receiver<OutgoingFrame<V>>;
/// Produces incoming frames.
///
/// Owned by a [`Channel`] that reads frames from the underlying transport and emits them to the
/// associated [`Connection`]. Paired with [`FrameReceiver`].
pub type FrameProducer<V> = mpmc::Sender<(Frame<V>, Callback<V>)>;

/// MAVLink connection.
#[derive(Debug)]
pub struct Connection<V: MaybeVersioned + 'static> {
    info: ConnectionInfo,
    sender: ConnSender<V>,
    receiver: ConnReceiver<V>,
    state: SharedCloser,
}

impl<V: MaybeVersioned> Connection<V> {
    /// Creates a new connection and associated [`ChannelBuilder`].
    pub fn new(info: ConnectionInfo, state: SharedCloser) -> (Self, ChannelBuilder<V>) {
        let (sender, send_handler) = mpmc::channel();
        let (producer, receiver) = mpmc::channel();

        let connection = Self {
            info,
            sender: ConnSender {
                state: state.as_closable(),
                sender: sender.clone(),
            },
            receiver: ConnReceiver { receiver },
            state,
        };

        let builder = ChannelBuilder {
            info: connection.info.clone(),
            state: connection.state.as_closable(),
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

    /// Close connection.
    pub fn close(&mut self) {
        if self.state.is_closed() {
            return;
        }
        self.state.close();
        log::debug!("[{:?}] connection closed", self.info);
    }

    pub(super) fn sender(&self) -> ConnSender<V> {
        self.sender.clone()
    }

    pub(super) fn receiver(&self) -> ConnReceiver<V> {
        self.receiver.clone()
    }
}

impl<V: MaybeVersioned> Drop for Connection<V> {
    fn drop(&mut self) {
        self.close();
    }
}

/// Builds a new channel for associated [`Connection`] interface.
#[derive(Clone, Debug)]
pub struct ChannelBuilder<V: MaybeVersioned + 'static> {
    info: ConnectionInfo,
    state: Closable,
    sender: FrameSender<V>,
    send_handler: FrameSendHandler<V>,
    producer: FrameProducer<V>,
}

impl<V: MaybeVersioned> ChannelBuilder<V> {
    /// Builds peer connection associated with [`Connection`] interface.
    #[must_use]
    pub fn build<R: Read, W: Write>(
        &self,
        info: PeerConnectionInfo,
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

    /// Information about associated [`Connection`] interface.
    pub fn info(&self) -> &ConnectionInfo {
        &self.info
    }

    /// Returns `true` if associated [`Connection`] interface is already closed.
    pub fn is_closed(&self) -> bool {
        self.state.is_closed()
    }
}

/// Channel associated with a particular [`Connection`].
///
/// Channels are constructed by [`ChannelBuilder`] associated with the corresponding connection.
pub struct Channel<V: MaybeVersioned + 'static, R: Read, W: Write> {
    conn_state: Closable,
    info: PeerConnectionInfo,
    reader: R,
    writer: W,
    sender: FrameSender<V>,
    send_handler: FrameSendHandler<V>,
    producer: FrameProducer<V>,
}

impl<V: MaybeVersioned + 'static, R: Read + Send + 'static, W: Write + Send + 'static>
    Channel<V, R, W>
{
    /// Spawn channel.
    ///
    /// Returns [`SharedCloser`] which can be used to control channel state. The state of the
    /// channel depends on the state of the associated [`Connection`]. However, it is not
    /// guaranteed, that channel will immediately close once connection is closed. There could be
    /// a lag relating to blocking nature of the underlying reader and writer.
    ///
    /// If caller is not interested in managing this channel, then it is required to drop returned
    /// [`SharedCloser`] or replace it with the corresponding [`Closable`].
    #[must_use]
    pub fn spawn(self) -> SharedCloser {
        let id = UniqueId::new();
        let info = Arc::new(self.info);
        let conn_state = self.conn_state.clone();
        let state = SharedCloser::new();

        log::trace!("[{info:?}] spawning peer connection");

        let write_handler = {
            let send_handler = self.send_handler;
            let frame_writer = Sender::new(self.writer);

            thread::spawn(move || Self::write_handler(id, send_handler, frame_writer))
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
        send_handler: FrameSendHandler<V>,
        mut frame_writer: Sender<W, V>,
    ) -> Result<()> {
        loop {
            let out_frame = send_handler.recv()?;

            if !out_frame.should_send_to(id) {
                continue;
            }

            if let Err(err) = frame_writer.send(out_frame.frame()) {
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

    fn read_handler(
        state: SharedCloser,
        conn_state: Closable,
        id: UniqueId,
        info: Arc<PeerConnectionInfo>,
        sender: FrameSender<V>,
        producer: FrameProducer<V>,
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

    fn handle_stop(
        state: SharedCloser,
        conn_state: Closable,
        info: Arc<PeerConnectionInfo>,
        write_handler: thread::JoinHandle<Result<()>>,
        read_handler: thread::JoinHandle<Result<()>>,
    ) {
        while !(state.is_closed()
            || conn_state.is_closed()
            || write_handler.is_finished()
            || read_handler.is_finished())
        {
            thread::sleep(PEER_CONN_STOP_POOLING_INTERVAL);
        }
        state.close();

        for i in 0..PEER_CONN_STOP_JOIN_ATTEMPTS {
            if write_handler.is_finished() && read_handler.is_finished() {
                break;
            }
            thread::sleep(PEER_CONN_STOP_JOIN_POOLING_INTERVAL);
            if i == PEER_CONN_STOP_JOIN_ATTEMPTS - 1 {
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

///////////////////////////////////////////////////////////////////////////////
//                                 PRIVATE                                   //
///////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Debug)]
pub(super) struct ConnSender<V: MaybeVersioned + 'static> {
    sender: FrameSender<V>,
    state: Closable,
}

impl<V: MaybeVersioned> ConnSender<V> {
    pub(super) fn send(&self, frame: &Frame<V>) -> Result<()> {
        if self.state.is_closed() {
            return Err(Error::from(mpsc::SendError(frame)));
        }

        self.sender
            .send(OutgoingFrame::new(frame.clone()))
            .map_err(Error::from)
    }
}

#[derive(Clone, Debug)]
pub(super) struct ConnReceiver<V: MaybeVersioned + 'static> {
    receiver: FrameReceiver<V>,
}

impl<V: MaybeVersioned> ConnReceiver<V> {
    pub(super) fn recv(&self) -> Result<(Frame<V>, Callback<V>)> {
        self.receiver.recv().map_err(Error::from)
    }

    pub(super) fn try_recv(&self) -> Result<(Frame<V>, Callback<V>)> {
        self.receiver.try_recv().map_err(Error::from)
    }
}
