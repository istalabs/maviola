use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use crate::core::io::{BroadcastScope, ConnectionInfo, OutgoingFrame};
use crate::core::marker::Proxy;
use crate::core::node::{NodeApi, NodeApiInternal};
use crate::core::utils::{Guarded, Sealed, SharedCloser, Switch};
use crate::error::SendError;
use crate::protocol::{DialectVersion, Endpoint, FrameProcessor, Peer};
use crate::sync::io::{Connection, ConnectionHandler};
use crate::sync::node::handler::{HeartbeatEmitter, InactivePeersHandler, IncomingFramesHandler};
use crate::sync::node::Event;

use crate::prelude::*;
use crate::sync::prelude::*;

/// <sup>[`sync`](crate::sync)</sup>
/// Synchronous API for MAVLink [`Node`].
pub struct SyncApi<V: MaybeVersioned> {
    connection: Connection<V>,
    sender: FrameSender<V, Proxy>,
    processor: Arc<FrameProcessor>,
    peers: Arc<RwLock<HashMap<MavLinkId, Peer>>>,
    event_sender: EventSender<V>,
    event_receiver: EventReceiver<V>,
}

impl<V: MaybeVersioned> Sealed for SyncApi<V> {}
impl<V: MaybeVersioned> NodeApiInternal<V> for SyncApi<V> {
    #[inline(always)]
    fn info(&self) -> &ConnectionInfo {
        self.connection.info()
    }

    unsafe fn route_frame_internal(&self, frame: Frame<V>, scope: BroadcastScope) -> Result<()> {
        self.sender
            .send_raw(OutgoingFrame::scoped(frame, scope))
            .map_err(Error::from)
    }

    #[inline(always)]
    fn processor_internal(&self) -> &FrameProcessor {
        self.processor()
    }
}
impl<V: MaybeVersioned> NodeApi<V> for SyncApi<V> {}
impl<V: MaybeVersioned> Debug for SyncApi<V> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SyncApi").finish_non_exhaustive()
    }
}

impl<V: MaybeVersioned> SyncApi<V> {
    pub(super) fn new(connection: Connection<V>, processor: Arc<FrameProcessor>) -> Self {
        let (events_tx, events_rx) = mpmc::channel();

        let sender = FrameSender::new(connection.sender().clone(), processor.clone());
        let event_receiver = EventReceiver::new(events_rx, connection.state(), processor.clone());

        SyncApi {
            connection,
            sender,
            processor: processor.clone(),
            peers: Arc::new(Default::default()),
            event_sender: EventSender::new(events_tx),
            event_receiver,
        }
    }

    #[inline(always)]
    fn processor(&self) -> &FrameProcessor {
        self.processor.as_ref()
    }

    #[inline(always)]
    pub(super) fn share_state(&self) -> SharedCloser {
        self.connection.share_state()
    }

    #[inline(always)]
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

    #[inline(always)]
    pub(super) fn frame_sender(&self) -> &FrameSender<V, Proxy> {
        &self.sender
    }

    #[inline(always)]
    pub(super) fn event_receiver(&self) -> &EventReceiver<V> {
        &self.event_receiver
    }

    pub(super) fn start_default_handlers(&self, heartbeat_timeout: Duration) {
        self.handle_incoming_frames();
        self.handle_inactive_peers(heartbeat_timeout);
    }

    pub(super) fn handle_conn_stop(&self, handler: ConnectionHandler) {
        handler.handle(&self.connection)
    }

    pub(super) fn connection(&self) -> &Connection<V> {
        &self.connection
    }

    fn handle_incoming_frames(&self) {
        let handler = IncomingFramesHandler {
            info: self.info().clone(),
            peers: self.peers.clone(),
            receiver: self.connection.receiver().clone(),
            event_sender: self.event_sender.clone(),
            sender: self.sender.clone(),
        };
        handler.spawn(self.connection.state());
    }

    fn handle_inactive_peers(&self, timeout: Duration) {
        let handler = InactivePeersHandler {
            info: self.info().clone(),
            peers: self.peers.clone(),
            timeout,
            event_sender: self.event_sender.clone(),
        };

        handler.spawn(self.connection.state());
    }
}

impl<V: Versioned> SyncApi<V> {
    pub(crate) fn start_sending_heartbeats(
        &self,
        endpoint: Endpoint<V>,
        interval: Duration,
        is_active: Guarded<SharedCloser, Switch>,
        dialect_version: Option<DialectVersion>,
    ) {
        let emitter = HeartbeatEmitter {
            info: self.info().clone(),
            endpoint,
            interval,
            sender: self.sender.clone(),
            dialect_version,
            _version: PhantomData::<V>,
        };
        emitter.spawn(is_active);
    }
}

///////////////////////////////////////////////////////////////////////////////
//                                 PRIVATE                                   //
///////////////////////////////////////////////////////////////////////////////

#[derive(Clone)]
pub(super) struct EventSender<V: MaybeVersioned> {
    inner: mpmc::Sender<Event<V>>,
}

impl<V: MaybeVersioned> EventSender<V> {
    pub(super) fn new(sender: mpmc::Sender<Event<V>>) -> Self {
        Self { inner: sender }
    }

    #[inline(always)]
    pub(super) fn send(&self, event: Event<V>) -> core::result::Result<(), SendError<Event<V>>> {
        self.inner.send(event)
    }
}
