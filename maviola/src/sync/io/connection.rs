use std::fmt::Debug;
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

use crate::core::error::{RecvError, RecvTimeoutError, SendError, TryRecvError};
use crate::core::io::{ConnectionConf, ConnectionInfo, OutgoingFrame};
use crate::core::utils::{Closable, SharedCloser};
use crate::sync::consts::CONN_STOP_POOLING_INTERVAL;
use crate::sync::io::{ChannelFactory, IncomingFrameReceiver, OutgoingFrameSender};
use crate::sync::marker::ConnConf;

use crate::prelude::*;
use crate::sync::prelude::*;

/// <sup>[`sync`](crate::sync)</sup>
/// Connection builder used to create a [`Connection`].
pub trait ConnectionBuilder<V: MaybeVersioned>: ConnectionConf {
    /// Builds connection from provided configuration.
    ///
    /// Returns the new connection and its main handler. Once handler is finished, the connection
    /// is considered to be closed.
    fn build(&self) -> Result<(Connection<V>, ConnectionHandler)>;

    /// Converts connection builder to [`ConnConf`]
    fn to_conf(&self) -> ConnConf<V>;

    /// If `true`, then this connection can be safely restored on failure.
    ///
    /// A blanket implementation always returns `false`.
    fn is_repairable(&self) -> bool {
        false
    }
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

    /// Spawns a new connection handler that finishes, when the `state` becomes closed.
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
            let result = self.inner.join();
            state.close();

            match result {
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

        let chan_factory = ChannelFactory {
            info: connection.info.clone(),
            state: connection.state.to_closable(),
            sender,
            send_handler,
            producer,
        };

        (connection, chan_factory)
    }

    /// Information about this connection.
    pub fn info(&self) -> &ConnectionInfo {
        &self.info
    }

    pub(in crate::sync) fn state(&self) -> Closable {
        self.state.to_closable()
    }

    pub(in crate::sync) fn share_state(&self) -> SharedCloser {
        self.state.clone()
    }

    pub(in crate::sync) fn sender(&self) -> &ConnSender<V> {
        &self.sender
    }

    pub(in crate::sync) fn receiver(&self) -> &ConnReceiver<V> {
        &self.receiver
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
pub(in crate::sync) struct ConnSender<V: MaybeVersioned + 'static> {
    sender: OutgoingFrameSender<V>,
    state: Closable,
}

impl<V: MaybeVersioned> ConnSender<V> {
    pub(in crate::sync) fn send(&self, frame: Frame<V>) -> Result<()> {
        if self.state.is_closed() {
            return Err(Error::from(SendError(frame)));
        }

        self.sender
            .send(OutgoingFrame::new(frame))
            .map_err(Error::from)
    }

    pub(in crate::sync) fn send_raw(
        &self,
        frame: OutgoingFrame<V>,
    ) -> core::result::Result<(), SendError<OutgoingFrame<V>>> {
        if self.state.is_closed() {
            return Err(SendError(frame));
        }

        self.sender.send(frame)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ConnReceiver<V: MaybeVersioned + 'static> {
    receiver: IncomingFrameReceiver<V>,
}

impl<V: MaybeVersioned> ConnReceiver<V> {
    #[inline(always)]
    pub(in crate::sync) fn recv(&self) -> core::result::Result<(Frame<V>, Callback<V>), RecvError> {
        self.receiver.recv()
    }

    #[inline(always)]
    pub(in crate::sync) fn recv_timeout(
        &self,
        timeout: Duration,
    ) -> core::result::Result<(Frame<V>, Callback<V>), RecvTimeoutError> {
        self.receiver.recv_timeout(timeout)
    }

    #[inline(always)]
    pub(in crate::sync) fn try_recv(
        &self,
    ) -> core::result::Result<(Frame<V>, Callback<V>), TryRecvError> {
        self.receiver.try_recv()
    }
}

///////////////////////////////////////////////////////////////////////////////
//                                  Tests                                    //
///////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::utils::net::pick_unused_port;
    use mavio::dialects::minimal::messages::Heartbeat;
    use std::time::Duration;

    #[test]
    fn standalone_connections() {
        let addr = format!("127.0.0.1:{}", pick_unused_port().unwrap());

        let (server, handler) = TcpServer::new(addr.as_str())
            .unwrap()
            .to_conf()
            .0
            .build()
            .unwrap();
        handler.handle::<V2>(&server);

        let (client, handler) = TcpClient::new(addr.as_str()).unwrap().build().unwrap();
        handler.handle::<V2>(&server);

        client.sender().send(make_frame()).unwrap();
        wait();
        server.receiver().try_recv().unwrap();

        server.sender().send(make_frame()).unwrap();
        wait();
        client.receiver().try_recv().unwrap();
    }

    fn wait() {
        thread::sleep(Duration::from_millis(100));
    }

    fn make_frame() -> Frame<V2> {
        Frame::builder()
            .sequence(0)
            .system_id(1)
            .component_id(1)
            .version(V2)
            .message(&Heartbeat::default())
            .unwrap()
            .build()
    }
}
