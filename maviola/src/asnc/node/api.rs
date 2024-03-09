use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;

use async_stream::stream;
use tokio::sync::RwLock;
use tokio_stream::Stream;

use crate::asnc::consts::CONN_BROADCAST_CHAN_CAPACITY;
use crate::asnc::io::{Callback, ConnReceiver, ConnSender, Connection, ConnectionHandler};
use crate::asnc::node::event::EventStream;
use crate::asnc::node::handler::{HeartbeatEmitter, InactivePeersHandler, IncomingFramesHandler};
use crate::asnc::node::Event;
use crate::core::error::{RecvError, RecvTimeoutError, SendError, TryRecvError};
use crate::core::io::{ConnectionInfo, OutgoingFrame};
use crate::core::node::NodeApi;
use crate::core::utils::{Guarded, Sealed, SharedCloser, Switch};
use crate::protocol::{DialectVersion, Endpoint, FrameProcessor, Peer};

use crate::asnc::prelude::*;
use crate::prelude::*;

/// <sup>[`async`](crate::asnc)</sup>
/// Synchronous API for MAVLink [`Node`].
pub struct AsyncApi<V: MaybeVersioned + 'static> {
    connection: Connection<V>,
    sender: FrameSender<V>,
    receiver: FrameReceiver<V>,
    processor: Arc<FrameProcessor>,
    peers: Arc<RwLock<HashMap<MavLinkId, Peer>>>,
    event_sender: EventSender<V>,
    event_receiver: EventReceiver<V>,
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

    #[inline(always)]
    fn processor(&self) -> &FrameProcessor {
        self.processor()
    }
}

impl<V: MaybeVersioned + 'static> AsyncApi<V> {
    pub(super) fn new(connection: Connection<V>, processor: Arc<FrameProcessor>) -> Self {
        let (events_tx, events_rx) = mpmc::channel(CONN_BROADCAST_CHAN_CAPACITY);

        let sender = FrameSender::new(connection.sender(), processor.clone());
        let receiver = FrameReceiver::new(connection.receiver(), processor.clone());
        let event_receiver = EventReceiver::new(events_rx).with_processor(processor.clone());

        AsyncApi {
            connection,
            sender,
            receiver,
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

    pub(super) fn send_frame(&self, frame: &Frame<V>) -> Result<()> {
        self.sender.send(frame)
    }

    pub(super) fn frame_sender(&self) -> &FrameSender<V> {
        &self.sender
    }

    pub(super) fn events(&self) -> impl Stream<Item = Event<V>> {
        EventStream::new(
            self.event_receiver.resubscribe(),
            self.connection.share_state().to_closable(),
        )
    }

    pub(super) async fn recv_event(&mut self) -> Result<Event<V>> {
        self.event_receiver.recv().await.map_err(Error::from)
    }

    pub(super) fn try_recv_event(&mut self) -> Result<Event<V>> {
        self.event_receiver.try_recv().map_err(Error::from)
    }

    pub(super) fn event_receiver(&self) -> EventReceiver<V> {
        self.event_receiver.clone()
    }

    pub(super) async fn recv_frame(&mut self) -> Result<(Frame<V>, Callback<V>)> {
        self.receiver.recv().await.map_err(Error::from)
    }

    pub(super) async fn recv_frame_timeout(
        &mut self,
        timeout: Duration,
    ) -> Result<(Frame<V>, Callback<V>)> {
        self.receiver
            .recv_timeout(timeout)
            .await
            .map_err(Error::from)
    }

    pub(super) fn try_recv_frame(&mut self) -> Result<(Frame<V>, Callback<V>)> {
        self.receiver.try_recv().map_err(Error::from)
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
            sender: self.connection.sender(),
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
pub(in crate::asnc) struct FrameSender<V: MaybeVersioned + 'static> {
    inner: ConnSender<V>,
    processor: Arc<FrameProcessor>,
}

pub(super) struct FrameReceiver<V: MaybeVersioned + 'static> {
    inner: ConnReceiver<V>,
    processor: Arc<FrameProcessor>,
}

#[derive(Clone)]
pub(super) struct EventSender<V: MaybeVersioned> {
    inner: mpmc::Sender<Event<V>>,
}

#[derive(Clone)]
pub(in crate::asnc) struct EventReceiver<V: MaybeVersioned + 'static> {
    inner: mpmc::Receiver<Event<V>>,
    processor: Arc<FrameProcessor>,
}

impl<V: MaybeVersioned + 'static> FrameSender<V> {
    pub(super) fn new(sender: ConnSender<V>, processor: Arc<FrameProcessor>) -> Self {
        Self {
            inner: sender,
            processor,
        }
    }

    pub(super) fn send(&self, frame: &Frame<V>) -> Result<()> {
        let mut frame = frame.clone();
        self.processor.process_outgoing(&mut frame)?;
        self.inner.send(frame)
    }

    pub(in crate::asnc) fn send_raw(
        &self,
        frame: OutgoingFrame<V>,
    ) -> core::result::Result<(), SendError<OutgoingFrame<V>>> {
        self.inner.send_raw(frame)
    }
}

impl<V: MaybeVersioned + 'static> FrameReceiver<V> {
    pub(super) fn new(receiver: ConnReceiver<V>, processor: Arc<FrameProcessor>) -> Self {
        Self {
            inner: receiver,
            processor,
        }
    }

    pub(super) async fn recv(
        &mut self,
    ) -> core::result::Result<(Frame<V>, Callback<V>), RecvError> {
        loop {
            return match self.inner.recv().await {
                Ok((mut frame, mut callback)) => {
                    if self.process_frame(&mut frame, &mut callback).is_err() {
                        continue;
                    }
                    Ok((frame, callback))
                }
                Err(err) => Err(err),
            };
        }
    }

    pub(super) async fn recv_timeout(
        &mut self,
        timeout: Duration,
    ) -> core::result::Result<(Frame<V>, Callback<V>), RecvTimeoutError> {
        loop {
            return match self.inner.recv_timeout(timeout).await {
                Ok((mut frame, mut callback)) => {
                    if self.process_frame(&mut frame, &mut callback).is_err() {
                        continue;
                    }
                    Ok((frame, callback))
                }
                Err(err) => Err(err),
            };
        }
    }

    pub(super) fn try_recv(
        &mut self,
    ) -> core::result::Result<(Frame<V>, Callback<V>), TryRecvError> {
        match self.inner.try_recv() {
            Ok((mut frame, mut callback)) => {
                if self.process_frame(&mut frame, &mut callback).is_err() {
                    return Err(TryRecvError::Empty);
                }
                Ok((frame, callback))
            }
            Err(err) => Err(err),
        }
    }

    fn process_frame(&self, frame: &mut Frame<V>, callback: &mut Callback<V>) -> Result<()> {
        callback.set_processor(self.processor.clone());
        self.processor.process_incoming(frame)?;
        Ok(())
    }
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

impl<V: MaybeVersioned> EventReceiver<V> {
    pub(super) fn new(receiver: mpmc::Receiver<Event<V>>) -> Self {
        Self {
            inner: receiver,
            processor: Arc::new(FrameProcessor::new()),
        }
    }

    fn with_processor(mut self, processor: Arc<FrameProcessor>) -> Self {
        self.processor = processor;
        self
    }

    pub(super) async fn recv(&mut self) -> core::result::Result<Event<V>, RecvError> {
        let event = self.inner.recv().await?;
        Ok(self.process_event(event))
    }

    pub(in crate::asnc) async fn recv_timeout(
        &mut self,
        timeout: Duration,
    ) -> core::result::Result<Event<V>, RecvTimeoutError> {
        let event = self.inner.recv_timeout(timeout).await?;
        Ok(self.process_event(event))
    }

    pub(super) fn try_recv(&mut self) -> core::result::Result<Event<V>, TryRecvError> {
        let event = self.inner.try_recv()?;
        Ok(self.process_event(event))
    }

    pub(super) fn resubscribe(&self) -> Self {
        Self::new(self.inner.resubscribe()).with_processor(self.processor.clone())
    }

    fn process_event(&self, event: Event<V>) -> Event<V> {
        match event {
            Event::Frame(mut frame, mut callback) => {
                callback.set_processor(self.processor.clone());

                if let Err(err) = self.processor.process_incoming(&mut frame) {
                    return Event::Invalid(frame, err, callback);
                }

                Event::Frame(frame, callback)
            }
            Event::Invalid(frame, err, mut callback) => {
                callback.set_processor(self.processor.clone());
                Event::Invalid(frame, err, callback)
            }
            Event::NewPeer(peer) => Event::NewPeer(peer),
            Event::PeerLost(peer) => Event::PeerLost(peer),
        }
    }
}
