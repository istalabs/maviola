use mavio::protocol::Behold;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio_stream::Stream;

use crate::asnc::node::event::EventStream;
use crate::core::utils::{Closable, Sealed};
use crate::error::{
    RecvError, RecvResult, RecvTimeoutError, RecvTimeoutResult, TryRecvError, TryRecvResult,
};
use crate::protocol::FrameProcessor;

use crate::asnc::prelude::*;
use crate::prelude::*;

/// <sup>[`async`](crate::asnc)</sup>
/// Node events receiver for asynchronous API.
///
/// **⚠** In order to have access to [`EventReceiver`] methods, you have to import
/// [`ReceiveEvent`] and [`ReceiveFrame`] traits. You may import [`asnc::prelude`] as well.
///
/// [`asnc::prelude`]: crate::asnc::prelude
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

    pub(in crate::asnc) fn state(&self) -> &Closable {
        &self.state
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

#[async_trait]
impl<V: MaybeVersioned> ReceiveEvent<V> for EventReceiver<V> {
    #[inline(always)]
    async fn recv(&mut self) -> RecvResult<Event<V>> {
        self.recv().await
    }

    #[inline(always)]
    async fn recv_timeout(&mut self, timeout: Duration) -> RecvTimeoutResult<Event<V>> {
        self.recv_timeout(timeout).await
    }

    #[inline(always)]
    fn try_recv(&mut self) -> TryRecvResult<Event<V>> {
        self.try_recv()
    }

    fn events(&self) -> Behold<impl Stream<Item = Event<V>>> {
        Behold::new(EventStream::new(self.clone()))
    }
}

#[async_trait]
impl<V: MaybeVersioned> ReceiveFrame<V> for EventReceiver<V> {}
