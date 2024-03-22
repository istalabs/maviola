use std::pin::Pin;
use std::task::{ready, Context, Poll};

use tokio_stream::Stream;
use tokio_util::sync::ReusableBoxFuture;

use crate::asnc::consts::EVENTS_RECV_POOLING_INTERVAL;
use crate::error::{FrameError, RecvError, TryRecvError};
use crate::protocol::Peer;

use crate::asnc::prelude::*;
use crate::prelude::*;

/// <sup>[`async`](crate::asnc)</sup>
/// Events.
#[derive(Clone, Debug)]
pub enum Event<V: MaybeVersioned> {
    /// New [`Peer`] appeared in the network.
    NewPeer(Peer),
    /// A [`Peer`] was lost due to the timeout.
    PeerLost(Peer),
    /// New [`Frame`] received.
    Frame(Frame<V>, Callback<V>),
    /// New [`Frame`] received, but it hasn't passed validation.
    Invalid(Frame<V>, FrameError, Callback<V>),
}

pub(crate) struct EventStream<V: MaybeVersioned> {
    inner: ReusableBoxFuture<'static, (RecvResult<V>, EventReceiver<V>)>,
}

type RecvResult<V> = core::result::Result<Event<V>, RecvError>;

impl<V: MaybeVersioned> EventStream<V> {
    pub(crate) fn new(rx: EventReceiver<V>) -> Self {
        Self {
            inner: ReusableBoxFuture::new(make_future(rx)),
        }
    }
}

async fn make_future<V: MaybeVersioned>(
    mut rx: EventReceiver<V>,
) -> (RecvResult<V>, EventReceiver<V>) {
    let handler = tokio::task::spawn(async move {
        let result = loop {
            if rx.state().is_closed() {
                break match rx.try_recv() {
                    Ok(event) => Ok(event),
                    Err(_) => Err(RecvError::Disconnected),
                };
            }

            break match rx.try_recv() {
                Ok(event) => Ok(event),
                Err(err) => match err {
                    TryRecvError::Empty => {
                        tokio::time::sleep(EVENTS_RECV_POOLING_INTERVAL).await;
                        continue;
                    }
                    TryRecvError::Disconnected => Err(RecvError::Disconnected),
                    TryRecvError::Lagged(n) => Err(RecvError::Lagged(n)),
                },
            };
        };

        (result, rx)
    });
    handler.await.unwrap()
}

impl<V: MaybeVersioned> Stream for EventStream<V> {
    type Item = Event<V>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let (result, rx) = ready!(self.inner.poll(cx));
        self.inner.set(make_future(rx));

        match result {
            Ok(event) => Poll::Ready(Some(event)),
            Err(err) => match err {
                RecvError::Disconnected => Poll::Ready(None),
                RecvError::Lagged(_) => {
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
            },
        }
    }
}

#[cfg(test)]
mod async_event_tests {
    use super::*;
    use crate::core::utils::Closer;
    use crate::protocol::FrameProcessor;
    use std::sync::Arc;
    use tokio_stream::StreamExt;

    #[tokio::test]
    async fn test_event_stream() {
        let state = Closer::new();
        let (tx, rx) = mpmc::channel(2);
        let event_receiver =
            EventReceiver::new(rx, state.to_closable(), Arc::new(FrameProcessor::default()));

        let mut stream: EventStream<V2> = EventStream::new(event_receiver);

        tx.send(Event::NewPeer(Peer::new(1, 1))).unwrap();
        tx.send(Event::NewPeer(Peer::new(1, 1))).unwrap();

        assert!(matches!(stream.next().await.unwrap(), Event::NewPeer(_)));
        assert!(matches!(stream.next().await.unwrap(), Event::NewPeer(_)));
    }
}
