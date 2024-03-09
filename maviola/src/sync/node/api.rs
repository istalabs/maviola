use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use crate::core::error::{RecvError, RecvTimeoutError, SendError, TryRecvError};
use crate::core::io::{ConnectionInfo, OutgoingFrame};
use crate::core::node::NodeApi;
use crate::core::utils::{Guarded, Sealed, SharedCloser, Switch};
use crate::protocol::{DialectVersion, Endpoint, FrameProcessor, Peer};
use crate::sync::io::{Callback, ConnReceiver, ConnSender, Connection, ConnectionHandler};
use crate::sync::node::event::EventsIterator;
use crate::sync::node::handler::{HeartbeatEmitter, InactivePeersHandler, IncomingFramesHandler};
use crate::sync::node::Event;

use crate::prelude::*;
use crate::sync::prelude::*;

/// <sup>[`sync`](crate::sync)</sup>
/// Synchronous API for MAVLink [`Node`].
pub struct SyncApi<V: MaybeVersioned + 'static> {
    connection: Connection<V>,
    sender: FrameSender<V>,
    receiver: FrameReceiver<V>,
    processor: Arc<FrameProcessor>,
    peers: Arc<RwLock<HashMap<MavLinkId, Peer>>>,
    event_sender: EventSender<V>,
    event_receiver: EventReceiver<V>,
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

    #[inline(always)]
    fn processor(&self) -> &FrameProcessor {
        self.processor()
    }
}

impl<V: MaybeVersioned + 'static> SyncApi<V> {
    pub(super) fn new(connection: Connection<V>, processor: Arc<FrameProcessor>) -> Self {
        let (events_tx, events_rx) = mpmc::channel();

        let sender = FrameSender::new(connection.sender().clone(), processor.clone());
        let receiver = FrameReceiver::new(connection.receiver().clone(), processor.clone());
        let event_receiver = EventReceiver::new(events_rx, processor.clone());

        SyncApi {
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
        self.processor.as_ref()
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

    pub(super) fn send_frame(&self, frame: &Frame<V>) -> Result<()> {
        self.sender.send(frame)
    }

    pub(super) fn frame_sender(&self) -> &FrameSender<V> {
        &self.sender
    }

    pub(super) fn events(&self) -> impl Iterator<Item = Event<V>> {
        EventsIterator::new(self.event_receiver.clone(), self.connection.state())
    }

    pub(super) fn recv_event(&self) -> Result<Event<V>> {
        self.event_receiver.recv().map_err(Error::from)
    }

    pub(super) fn recv_event_timeout(&self, timeout: Duration) -> Result<Event<V>> {
        self.event_receiver
            .recv_timeout(timeout)
            .map_err(Error::from)
    }

    pub(super) fn try_recv_event(&self) -> Result<Event<V>> {
        self.event_receiver.try_recv().map_err(Error::from)
    }

    pub(super) fn event_receiver(&self) -> &EventReceiver<V> {
        &self.event_receiver
    }

    pub(super) fn recv_frame(&self) -> Result<(Frame<V>, Callback<V>)> {
        self.receiver.recv().map_err(Error::from)
    }

    pub(super) fn recv_frame_timeout(&self, timeout: Duration) -> Result<(Frame<V>, Callback<V>)> {
        self.receiver.recv_timeout(timeout).map_err(Error::from)
    }

    pub(super) fn try_recv_frame(&self) -> Result<(Frame<V>, Callback<V>)> {
        self.receiver.try_recv().map_err(Error::from)
    }

    pub(super) fn start_default_handlers(&self, heartbeat_timeout: Duration) {
        self.handle_incoming_frames();
        self.handle_inactive_peers(heartbeat_timeout);
    }

    pub(super) fn handle_conn_stop(&self, handler: ConnectionHandler) {
        handler.handle(&self.connection)
    }

    fn handle_incoming_frames(&self) {
        let handler = IncomingFramesHandler {
            info: self.info().clone(),
            peers: self.peers.clone(),
            receiver: self.connection.receiver().clone(),
            event_sender: self.event_sender.clone(),
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
pub(in crate::sync) struct FrameSender<V: MaybeVersioned + 'static> {
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
pub(in crate::sync) struct EventReceiver<V: MaybeVersioned + 'static> {
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

    pub(in crate::sync) fn send_raw(
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

    pub(super) fn recv(&self) -> core::result::Result<(Frame<V>, Callback<V>), RecvError> {
        loop {
            return match self.inner.recv() {
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

    pub(super) fn recv_timeout(
        &self,
        timeout: Duration,
    ) -> core::result::Result<(Frame<V>, Callback<V>), RecvTimeoutError> {
        loop {
            return match self.inner.recv_timeout(timeout) {
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

    pub(super) fn try_recv(&self) -> core::result::Result<(Frame<V>, Callback<V>), TryRecvError> {
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

    #[inline(always)]
    pub(super) fn send(&self, event: Event<V>) -> core::result::Result<(), SendError<Event<V>>> {
        self.inner.send(event)
    }
}

impl<V: MaybeVersioned> EventReceiver<V> {
    pub(super) fn new(receiver: mpmc::Receiver<Event<V>>, processor: Arc<FrameProcessor>) -> Self {
        Self {
            inner: receiver,
            processor,
        }
    }

    pub(super) fn recv(&self) -> core::result::Result<Event<V>, RecvError> {
        Ok(self.process_event(self.inner.recv()?))
    }

    pub(in crate::sync) fn recv_timeout(
        &self,
        timeout: Duration,
    ) -> core::result::Result<Event<V>, RecvTimeoutError> {
        Ok(self.process_event(self.inner.recv_timeout(timeout)?))
    }

    pub(super) fn try_recv(&self) -> core::result::Result<Event<V>, TryRecvError> {
        Ok(self.process_event(self.inner.try_recv()?))
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
