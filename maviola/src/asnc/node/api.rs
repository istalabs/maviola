use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;

use async_stream::stream;
use tokio::sync::RwLock;
use tokio_stream::Stream;

use crate::asnc::consts::CONN_BROADCAST_CHAN_CAPACITY;
use crate::asnc::io::{Callback, Connection, ConnectionHandler};
use crate::asnc::node::event::EventStream;
use crate::asnc::node::handler::{HeartbeatEmitter, InactivePeersHandler, IncomingFramesHandler};
use crate::asnc::node::Event;
use crate::core::io::ConnectionInfo;
use crate::core::node::NodeApi;
use crate::core::utils::{Guarded, Sealed, SharedCloser, Switch};
use crate::protocol::{Endpoint, Peer};

use crate::prelude::*;

/// <sup>[`async`](crate::asnc)</sup>
/// Synchronous API for MAVLink [`Node`].
pub struct AsyncApi<V: MaybeVersioned + 'static> {
    connection: Connection<V>,
    peers: Arc<RwLock<HashMap<MavLinkId, Peer>>>,
    events_tx: broadcast::Sender<Event<V>>,
    events_rx: broadcast::Receiver<Event<V>>,
}

impl<V: MaybeVersioned + 'static> Sealed for AsyncApi<V> {}
impl<V: MaybeVersioned + 'static> NodeApi<V> for AsyncApi<V> {
    #[inline(always)]
    fn info(&self) -> &ConnectionInfo {
        self.connection.info()
    }

    #[inline(always)]
    fn send_frame(&self, frame: &Frame<V>) -> Result<()> {
        self.send_frame(frame)
    }
}

impl<V: MaybeVersioned + 'static> AsyncApi<V> {
    pub(super) fn new(connection: Connection<V>) -> Self {
        let (events_tx, events_rx) = broadcast::channel(CONN_BROADCAST_CHAN_CAPACITY);

        AsyncApi {
            connection,
            peers: Arc::new(Default::default()),
            events_tx,
            events_rx,
        }
    }

    pub(super) fn share_state(&self) -> SharedCloser {
        self.connection.share_state()
    }

    pub(super) fn info(&self) -> &ConnectionInfo {
        self.connection.info()
    }

    pub(super) async fn peers(&self) -> impl Stream<Item = Peer> {
        let peers = self
            .peers
            .read()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();

        stream! {
            for peer in peers {
                yield peer;
            }
        }
    }

    pub(super) async fn has_peers(&self) -> bool {
        !self.peers.read().await.is_empty()
    }

    pub(super) fn events(&self) -> impl Stream<Item = Event<V>> {
        EventStream::new(
            self.events_rx.resubscribe(),
            self.connection.share_state().to_closable(),
        )
    }

    pub(super) async fn start_default_handlers(&self, heartbeat_timeout: Duration) {
        self.handle_incoming_frames().await;
        self.handle_inactive_peers(heartbeat_timeout).await;
    }

    pub(super) async fn handle_conn_stop(&self, handler: ConnectionHandler) {
        handler.handle(&self.connection);
    }

    async fn handle_incoming_frames(&self) {
        let handler = IncomingFramesHandler {
            info: self.info().clone(),
            peers: self.peers.clone(),
            receiver: self.connection.receiver(),
            events_tx: self.events_tx.clone(),
        };
        handler
            .spawn(self.connection.share_state().to_closable())
            .await;
    }

    async fn handle_inactive_peers(&self, timeout: Duration) {
        let handler = InactivePeersHandler {
            info: self.info().clone(),
            peers: self.peers.clone(),
            timeout,
            events_tx: self.events_tx.clone(),
        };

        handler
            .spawn(self.connection.share_state().to_closable())
            .await;
    }

    pub(super) async fn recv_frame(&mut self) -> Result<(Frame<V>, Callback<V>)> {
        self.connection.recv().await
    }

    pub(super) fn try_recv_frame(&mut self) -> Result<(Frame<V>, Callback<V>)> {
        self.connection.try_recv()
    }

    pub(super) fn send_frame(&self, frame: &Frame<V>) -> Result<()> {
        self.connection.send(frame).map(|_| ())
    }

    pub(super) async fn recv_event(&mut self) -> Result<Event<V>> {
        self.events_rx.recv().await.map_err(Error::from)
    }

    pub(super) fn try_recv_event(&mut self) -> Result<Event<V>> {
        self.events_rx.try_recv().map_err(Error::from)
    }
}

impl<V: Versioned> AsyncApi<V> {
    pub(crate) async fn start_sending_heartbeats<D: Dialect>(
        &self,
        endpoint: Endpoint<V>,
        interval: Duration,
        is_active: Guarded<SharedCloser, Switch>,
    ) {
        let emitter = HeartbeatEmitter {
            info: self.info().clone(),
            endpoint,
            interval,
            sender: self.connection.sender(),
            _dialect: PhantomData::<D>,
            _version: PhantomData::<V>,
        };
        emitter.spawn(is_active).await;
    }
}
