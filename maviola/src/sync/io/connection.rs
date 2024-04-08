use std::fmt::Debug;
use std::thread;
use std::thread::JoinHandle;

use crate::core::io::{ConnectionConf, ConnectionInfo};
use crate::core::utils::{Closable, SharedCloser};
use crate::sync::consts::CONN_STOP_POOLING_INTERVAL;
use crate::sync::io::{
    incoming_channel, outgoing_channel, ChannelFactory, IncomingFrameReceiver, OutgoingFrameSender,
};
use crate::sync::marker::ConnConf;

use crate::prelude::*;

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
pub struct Connection<V: MaybeVersioned> {
    info: ConnectionInfo,
    sender: OutgoingFrameSender<V>,
    receiver: IncomingFrameReceiver<V>,
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
        let (sender, send_handler) = outgoing_channel(state.to_closable());
        let (producer, receiver) = incoming_channel();

        let connection = Self {
            info,
            sender: sender.clone(),
            receiver,
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

    pub(in crate::sync) fn sender(&self) -> &OutgoingFrameSender<V> {
        &self.sender
    }

    pub(in crate::sync) fn receiver(&self) -> &IncomingFrameReceiver<V> {
        &self.receiver
    }

    pub(in crate::sync) fn reuse(&self) -> Self {
        let mut state = SharedCloser::new();

        let conn = Self {
            info: self.info.clone(),
            sender: self.sender.clone(),
            receiver: self.receiver.clone(),
            state: state.clone(),
        };

        let parent_state = self.state.to_closable();

        thread::spawn(move || {
            while !parent_state.is_closed() && !state.is_closed() {
                thread::sleep(CONN_STOP_POOLING_INTERVAL);
            }
            state.close();
        });

        conn
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
//                                  Tests                                    //
///////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::utils::net::pick_unused_port;
    use crate::dialects::minimal::messages::Heartbeat;
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
