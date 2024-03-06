use std::future::Future;
use std::pin::Pin;
use std::task::{ready, Context, Poll};

use tokio::sync::broadcast::error::{RecvError, TryRecvError};
use tokio_stream::Stream;
use tokio_util::sync::ReusableBoxFuture;

use crate::asnc::consts::EVENTS_RECV_POOLING_INTERVAL;
use crate::asnc::io::Callback;
use crate::core::utils::Closable;
use crate::protocol::Peer;

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
}

pub(crate) struct EventStream<V: MaybeVersioned + 'static> {
    inner: ReusableBoxFuture<'static, (RecvResult<V>, EventReceiver<V>, Closable)>,
}

type RecvResult<V> = core::result::Result<Event<V>, RecvError>;
type EventReceiver<V> = broadcast::Receiver<Event<V>>;

impl<V: MaybeVersioned + 'static> EventStream<V> {
    pub(crate) fn new(rx: broadcast::Receiver<Event<V>>, state: Closable) -> Self {
        Self {
            inner: ReusableBoxFuture::new(make_future(rx, state)),
        }
    }
}

async fn make_future<V: MaybeVersioned + 'static>(
    mut rx: broadcast::Receiver<Event<V>>,
    state: Closable,
) -> (RecvResult<V>, EventReceiver<V>, Closable) {
    let handler = tokio::task::spawn(async move {
        let result = loop {
            if state.is_closed() {
                break match rx.try_recv() {
                    Ok(event) => Ok(event),
                    Err(_) => Err(RecvError::Closed),
                };
            }

            break match rx.try_recv() {
                Ok(event) => Ok(event),
                Err(err) => match err {
                    TryRecvError::Empty => {
                        tokio::time::sleep(EVENTS_RECV_POOLING_INTERVAL).await;
                        continue;
                    }
                    TryRecvError::Closed => Err(RecvError::Closed),
                    TryRecvError::Lagged(n) => Err(RecvError::Lagged(n)),
                },
            };
        };

        (result, rx, state)
    });
    handler.await.unwrap()
}

impl<V: MaybeVersioned + 'static> Stream for EventStream<V> {
    type Item = Event<V>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let (result, rx, state) = ready!(self.inner.poll(cx));
        self.inner.set(make_future(rx, state));

        match result {
            Ok(event) => Poll::Ready(Some(event)),
            Err(err) => match err {
                RecvError::Closed => Poll::Ready(None),
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
    use tokio_stream::StreamExt;

    #[tokio::test]
    async fn test_event_stream() {
        let (tx, rx) = broadcast::channel(2);
        let state = Closer::new();

        let mut stream: EventStream<V2> = EventStream::new(rx, state.to_closable());

        tx.send(Event::NewPeer(Peer::new(1, 1))).unwrap();
        tx.send(Event::NewPeer(Peer::new(1, 1))).unwrap();

        assert!(matches!(stream.next().await.unwrap(), Event::NewPeer(_)));
        assert!(matches!(stream.next().await.unwrap(), Event::NewPeer(_)));
    }
}
