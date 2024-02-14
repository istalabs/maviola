use std::fmt::Debug;
use std::io::{Read, Write};
use std::net::SocketAddr;
use std::sync::atomic::AtomicBool;
use std::sync::{atomic, mpsc, Arc};
use std::thread;

use mavio::protocol::MaybeVersioned;
use mavio::{Frame, Receiver, Sender};

use crate::io::sync::response::{BroadcastScope, Response, ResponseFrame};
use crate::utils::UniqueId;

use crate::prelude::*;

/// Information about a connection.
#[derive(Clone, Debug)]
pub enum ConnectionInfo {
    /// TCP server.
    TcpServer {
        /// Server address.
        bind_addr: SocketAddr,
    },
    /// TCP client.
    TcpClient {
        /// Server address.
        remote_addr: SocketAddr,
    },
}

/// Information about a peer connection.
///
/// A particular [`Connection`] may have several peer connection. For example, a TCP server creates
/// a peer connection for each client.
#[derive(Clone, Debug)]
pub enum PeerConnectionInfo {
    /// TCP server.
    TcpServer {
        /// Server address.
        server_addr: SocketAddr,
        /// Peer address.
        peer_addr: SocketAddr,
    },
    /// TCP client.
    TcpClient {
        /// Server address.
        server_addr: SocketAddr,
    },
}

/// Connection configuration.
pub trait ConnectionConf<V: MaybeVersioned>: ConnectionBuilder<V> {
    fn info(&self) -> &ConnectionInfo;
}

/// Connection builder used to create a [`Connection`].
pub trait ConnectionBuilder<V: MaybeVersioned>: Debug + Send {
    /// Builds [`Connection`] from provided configuration.
    fn build(&self) -> Result<Connection<V>>;
}

/// Synchronous MAVLink connection.
#[derive(Clone, Debug)]
pub struct Connection<V: MaybeVersioned + 'static> {
    info: ConnectionInfo,
    sender: FrameSender<V>,
    receiver: FrameReceiver<V>,
    is_active: Arc<AtomicBool>,
}

impl<V: MaybeVersioned> Connection<V> {
    pub(crate) fn new(
        info: ConnectionInfo,
        sender: FrameSender<V>,
        receiver: FrameReceiver<V>,
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
    pub fn send(&self, frame: &Frame<V>) -> Result<()> {
        if !self.is_active.load(atomic::Ordering::Relaxed) {
            return Err(Error::from(mpsc::SendError(frame)));
        }

        let frame = Arc::new(frame.clone());
        self.sender
            .send(ResponseFrame { frame, scope: None })
            .map_err(Error::from)
    }

    /// Receive frame.
    ///
    /// Blocks until frame received.
    pub fn recv(&self) -> Result<(Frame<V>, Response<V>)> {
        if !self.is_active.load(atomic::Ordering::Relaxed) {
            return Err(Error::from(mpsc::RecvError));
        }

        self.receiver.recv().map_err(Error::from)
    }

    /// Attempts to receive MAVLink frame without blocking.
    pub fn try_recv(&self) -> Result<(Frame<V>, Response<V>)> {
        if !self.is_active.load(atomic::Ordering::Relaxed) {
            return Err(Error::from(mpsc::TryRecvError::Disconnected));
        }

        self.receiver.try_recv().map_err(Error::from)
    }

    /// Close connection.
    pub fn close(&mut self) {
        self.is_active.store(false, atomic::Ordering::Relaxed);
        self.sender.disconnect();
        self.receiver.disconnect();
    }
}

///////////////////////////////////////////////////////////////////////////////
//                                 PRIVATE                                   //
///////////////////////////////////////////////////////////////////////////////

pub(crate) type FrameSender<V> = mpmc::Sender<ResponseFrame<V>>;
pub(crate) type FrameReceiver<V> = mpmc::Receiver<(Frame<V>, Response<V>)>;

pub(crate) struct PeerConnection<V: MaybeVersioned + 'static, R: Read, W: Write> {
    pub(crate) info: PeerConnectionInfo,
    pub(crate) reader: R,
    pub(crate) writer: W,
    pub(crate) send_tx: mpmc::Sender<ResponseFrame<V>>,
    pub(crate) send_rx: mpmc::Receiver<ResponseFrame<V>>,
    pub(crate) recv_tx: mpmc::Sender<(Frame<V>, Response<V>)>,
}

impl<V: MaybeVersioned + 'static, R: Read + Send + 'static, W: Write + Send + 'static>
    PeerConnection<V, R, W>
{
    pub(crate) fn start(self) {
        let id = UniqueId::new();
        let info = Arc::new(self.info);

        {
            let info = info.clone();
            let send_rx = self.send_rx;
            let sender = Sender::new(self.writer);

            thread::spawn(move || {
                Self::send_handler(id, info, send_rx, sender);
            });
        }

        {
            let info = info.clone();
            let send_tx = self.send_tx;
            let recv_tx = self.recv_tx;
            let receiver = Receiver::new(self.reader);

            thread::spawn(move || Self::recv_handler(id, info, send_tx, recv_tx, receiver));
        }
    }

    fn send_handler(
        id: UniqueId,
        info: Arc<PeerConnectionInfo>,
        send_rx: mpmc::Receiver<ResponseFrame<V>>,
        mut sender: Sender<W, V>,
    ) {
        loop {
            let resp_frame = match send_rx.recv() {
                Ok(frame) => frame,
                Err(err) => {
                    log::error!("[{info:?}] can't receive outgoing frame: {err:?}");
                    return;
                }
            };

            let (frame, scope) = (resp_frame.frame, resp_frame.scope);

            let should_send = match scope {
                None => true,
                Some(scope) => match scope {
                    BroadcastScope::All => true,
                    BroadcastScope::Except(sender_id) => sender_id != id,
                    BroadcastScope::Exact(sender_id) => sender_id == id,
                },
            };
            if !should_send {
                continue;
            }

            if let Err(err) = sender.send(&frame) {
                log::trace!("[{info:?}] can't send a frame: {err:?}");

                let err = Error::from(err);
                if let Error::Io(err) = err {
                    log::trace!("[{info:?}] I/O error sending frame: {err:?}");
                    return;
                }
            }
        }
    }

    fn recv_handler(
        id: UniqueId,
        info: Arc<PeerConnectionInfo>,
        send_tx: mpmc::Sender<ResponseFrame<V>>,
        recv_tx: mpmc::Sender<(Frame<V>, Response<V>)>,
        mut receiver: Receiver<R, V>,
    ) {
        loop {
            let info = info.clone();
            let send_tx = send_tx.clone();

            let frame = match receiver.recv() {
                Ok(frame) => frame,
                Err(err) => {
                    log::trace!("[{info:?}] can't receive incoming frame: {err:?}");

                    let err = Error::from(err);
                    if let Error::Io(err) = err {
                        log::trace!("[{info:?}] I/O error receiving frame: {err:?}");
                        return;
                    }
                    continue;
                }
            };

            let response = Response {
                sender_id: id,
                sender_info: info.clone(),
                broadcast_tx: send_tx,
            };

            if let Err(err) = recv_tx.send((frame, response)) {
                log::trace!("[{info:?}] can't pass incoming frame: {err:?}");
                return;
            }
        }
    }
}
