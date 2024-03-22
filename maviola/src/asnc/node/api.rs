use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;

use async_stream::stream;
use tokio::sync::RwLock;
use tokio_stream::Stream;

use crate::asnc::consts::CONN_BROADCAST_CHAN_CAPACITY;
use crate::asnc::io::{Connection, ConnectionHandler};
use crate::asnc::node::event::EventStream;
use crate::asnc::node::handler::{HeartbeatEmitter, InactivePeersHandler, IncomingFramesHandler};
use crate::asnc::node::Event;
use crate::core::io::{BroadcastScope, ConnectionInfo, OutgoingFrame};
use crate::core::marker::Proxy;
use crate::core::node::{NodeApi, NodeApiInternal};
use crate::core::utils::{Guarded, Sealed, SharedCloser, Switch};
use crate::error::SendError;
use crate::protocol::{DialectVersion, Endpoint, FrameProcessor, Peer};

use crate::asnc::prelude::*;
use crate::prelude::*;

/// <sup>[`async`](crate::asnc)</sup>
/// Synchronous API for MAVLink [`Node`].
pub struct AsyncApi<V: MaybeVersioned> {
    connection: Connection<V>,
    sender: FrameSender<V, Proxy>,
    processor: Arc<FrameProcessor>,
    peers: Arc<RwLock<HashMap<MavLinkId, Peer>>>,
    event_sender: EventSender<V>,
    event_receiver: EventReceiver<V>,
}

impl<V: MaybeVersioned> Sealed for AsyncApi<V> {}
impl<V: MaybeVersioned> NodeApiInternal<V> for AsyncApi<V> {
    #[inline(always)]
    fn info(&self) -> &ConnectionInfo {
        self.connection.info()
    }

    #[inline(always)]
    unsafe fn route_frame_internal(&self, frame: Frame<V>, scope: BroadcastScope) -> Result<()> {
        self.route_frame_internal(frame, scope)
    }

    #[inline(always)]
    fn processor(&self) -> &FrameProcessor {
        self.processor()
    }
}
impl<V: MaybeVersioned> NodeApi<V> for AsyncApi<V> {}
impl<V: MaybeVersioned> Debug for AsyncApi<V> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AsyncApi").finish_non_exhaustive()
    }
}

impl<V: MaybeVersioned> AsyncApi<V> {
    pub(super) fn new(connection: Connection<V>, processor: Arc<FrameProcessor>) -> Self {
        let (events_tx, events_rx) = mpmc::channel(CONN_BROADCAST_CHAN_CAPACITY);

        let sender = FrameSender::new(connection.sender(), processor.clone());
        let event_receiver = EventReceiver::new(events_rx, connection.state(), processor.clone());

        AsyncApi {
            connection,
            sender,
            processor: processor.clone(),
            peers: Arc::new(Default::default()),
            event_sender: EventSender::new(events_tx),
            event_receiver,
        }
    }

    fn processor(&self) -> &FrameProcessor {
        &self.processor
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

    unsafe fn route_frame_internal(&self, frame: Frame<V>, scope: BroadcastScope) -> Result<()> {
        self.sender
            .send_raw(OutgoingFrame::scoped(frame, scope))
            .map_err(Error::from)
    }

    pub(super) fn frame_sender(&self) -> &FrameSender<V, Proxy> {
        &self.sender
    }

    pub(super) fn events(&self) -> impl Stream<Item = Event<V>> {
        EventStream::new(self.event_receiver.clone())
    }

    pub(super) fn event_receiver(&self) -> &EventReceiver<V> {
        &self.event_receiver
    }

    pub(super) fn event_receiver_mut(&mut self) -> &mut EventReceiver<V> {
        &mut self.event_receiver
    }

    pub(super) async fn start_default_handlers(&self, heartbeat_timeout: Duration) {
        self.handle_incoming_frames();
        self.handle_inactive_peers(heartbeat_timeout);
    }

    pub(super) async fn handle_conn_stop(&self, handler: ConnectionHandler) {
        handler.handle(&self.connection);
    }

    fn handle_incoming_frames(&self) {
        let handler = IncomingFramesHandler {
            info: self.info().clone(),
            peers: self.peers.clone(),
            receiver: self.connection.receiver(),
            event_sender: self.event_sender.clone(),
            sender: self.sender.clone(),
        };
        handler.spawn(self.connection.share_state().to_closable());
    }

    fn handle_inactive_peers(&self, timeout: Duration) {
        let handler = InactivePeersHandler {
            info: self.info().clone(),
            peers: self.peers.clone(),
            timeout,
            event_sender: self.event_sender.clone(),
        };

        handler.spawn(self.connection.share_state().to_closable());
    }
}

impl<V: Versioned> AsyncApi<V> {
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

    #[inline]
    pub(super) fn send(&self, event: Event<V>) -> core::result::Result<(), SendError<Event<V>>> {
        self.inner.send(event).map_err(SendError::from).map(|_| ())
    }
}
