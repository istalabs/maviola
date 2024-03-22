use std::sync::Arc;
use std::time::Duration;

use crate::core::utils::{Closable, Sealed};
use crate::error::{
    RecvError, RecvResult, RecvTimeoutError, RecvTimeoutResult, TryRecvError, TryRecvResult,
};
use crate::protocol::FrameProcessor;
use crate::sync::node::event::EventsIterator;

use crate::prelude::*;
use crate::sync::prelude::*;

/// <sup>[`sync`](crate::sync)</sup>
/// Node events receiver for synchronous API.
#[derive(Clone)]
pub struct EventReceiver<V: MaybeVersioned> {
    inner: mpmc::Receiver<Event<V>>,
    state: Closable,
    processor: Arc<FrameProcessor>,
}

impl<V: MaybeVersioned> EventReceiver<V> {
    pub(super) fn new(
        receiver: mpmc::Receiver<Event<V>>,
        state: Closable,
        processor: Arc<FrameProcessor>,
    ) -> Self {
        Self {
            inner: receiver,
            state,
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

impl<V: MaybeVersioned> Sealed for EventReceiver<V> {}

impl<V: MaybeVersioned> ReceiveEvent<V> for EventReceiver<V> {
    #[inline(always)]
    fn recv(&self) -> RecvResult<Event<V>> {
        self.recv()
    }

    #[inline(always)]
    fn recv_timeout(&self, timeout: Duration) -> RecvTimeoutResult<Event<V>> {
        self.recv_timeout(timeout)
    }

    #[inline(always)]
    fn try_recv(&self) -> TryRecvResult<Event<V>> {
        self.try_recv()
    }

    fn events(&self) -> impl Iterator<Item = Event<V>> {
        EventsIterator::new(self.clone(), self.state.clone())
    }
}
