use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use crate::core::io::ConnectionInfo;
use crate::core::node::NodeApi;
use crate::core::utils::{Guarded, Sealed, SharedCloser, Switch};
use crate::protocol::{Endpoint, Peer};
use crate::sync::io::{Callback, Connection};
use crate::sync::node::event::EventsIterator;
use crate::sync::node::handler::{HeartbeatEmitter, InactivePeersHandler, IncomingFramesHandler};
use crate::sync::node::Event;

use crate::prelude::*;

/// <sup>[`sync`](crate::sync)</sup>
/// Synchronous API for MAVLink [`Node`].
pub struct SyncApi<V: MaybeVersioned + 'static> {
    state: SharedCloser,
    connection: Connection<V>,
    peers: Arc<RwLock<HashMap<MavLinkId, Peer>>>,
    events_tx: mpmc::Sender<Event<V>>,
    events_rx: mpmc::Receiver<Event<V>>,
}

impl<V: MaybeVersioned + 'static> Sealed for SyncApi<V> {}
impl<V: MaybeVersioned + 'static> NodeApi<V> for SyncApi<V> {
    #[inline(always)]
    fn info(&self) -> &ConnectionInfo {
        self.connection.info()
    }

    #[inline(always)]
    fn send_frame(&self, frame: &Frame<V>) -> Result<()> {
        self.send_frame(frame)
    }
}

impl<V: MaybeVersioned + 'static> SyncApi<V> {
    pub(super) fn new(connection: Connection<V>) -> Self {
        let (events_tx, events_rx) = mpmc::channel();

        SyncApi {
            state: connection.share_state(),
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

    pub(super) fn peers(&self) -> impl Iterator<Item = Peer> {
        let peers: Vec<Peer> = match self.peers.read() {
            Ok(peers) => peers.values().cloned().collect(),
            Err(_) => Vec::new(),
        };

        peers.into_iter()
    }

    pub(super) fn has_peers(&self) -> bool {
        match self.peers.read() {
            Ok(peers) => !peers.is_empty(),
            Err(_) => false,
        }
    }

    pub(super) fn events(&self) -> impl Iterator<Item = Event<V>> {
        EventsIterator {
            rx: self.events_rx.clone(),
        }
    }

    pub(super) fn start_default_handlers(&self, heartbeat_timeout: Duration) {
        self.handle_incoming_frames();
        self.handle_inactive_peers(heartbeat_timeout);
    }

    fn handle_incoming_frames(&self) {
        let handler = IncomingFramesHandler {
            info: self.info().clone(),
            peers: self.peers.clone(),
            receiver: self.connection.receiver(),
            events_tx: self.events_tx.clone(),
        };
        handler.spawn(self.state.to_closable());
    }

    fn handle_inactive_peers(&self, timeout: Duration) {
        let handler = InactivePeersHandler {
            info: self.info().clone(),
            peers: self.peers.clone(),
            timeout,
            events_tx: self.events_tx.clone(),
        };

        handler.spawn(self.state.to_closable());
    }

    pub(super) fn recv_frame(&self) -> Result<(Frame<V>, Callback<V>)> {
        self.connection.recv()
    }

    pub(super) fn try_recv_frame(&self) -> Result<(Frame<V>, Callback<V>)> {
        self.connection.try_recv()
    }

    pub(super) fn send_frame(&self, frame: &Frame<V>) -> Result<()> {
        self.connection.send(frame)
    }

    pub(super) fn recv_event(&self) -> Result<Event<V>> {
        self.events_rx.recv().map_err(Error::from)
    }

    pub(super) fn try_recv_event(&self) -> Result<Event<V>> {
        self.events_rx.try_recv().map_err(Error::from)
    }
}

impl<V: Versioned> SyncApi<V> {
    pub(crate) fn start_sending_heartbeats<D: Dialect>(
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
        emitter.spawn(is_active);
    }
}
