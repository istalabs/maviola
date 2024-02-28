use std::future::Future;
use std::pin::Pin;
use std::task::{ready, Context, Poll};

use tokio::sync::broadcast::error::RecvError;
use tokio_stream::Stream;
use tokio_util::sync::ReusableBoxFuture;

use crate::asnc::io::Callback;
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
    inner: ReusableBoxFuture<'static, (RecvResult<V>, EventReceiver<V>)>,
}

type RecvResult<V> = core::result::Result<Event<V>, RecvError>;
type EventReceiver<V> = broadcast::Receiver<Event<V>>;

impl<V: MaybeVersioned + 'static> EventStream<V> {
    pub(crate) fn new(rx: broadcast::Receiver<Event<V>>) -> Self {
        Self {
            inner: ReusableBoxFuture::new(make_future(rx)),
        }
    }
}

async fn make_future<V: MaybeVersioned + 'static>(
    mut rx: broadcast::Receiver<Event<V>>,
) -> (RecvResult<V>, EventReceiver<V>) {
    let result = rx.recv().await;
    (result, rx)
}

impl<V: MaybeVersioned + 'static> Stream for EventStream<V> {
    type Item = Event<V>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let (result, rx) = ready!(self.inner.poll(cx));
        self.inner.set(make_future(rx));

        match result {
            Ok(event) => Poll::Ready(Some(event)),
            Err(err) => match err {
                RecvError::Closed => Poll::Ready(None),
                RecvError::Lagged(_) => Poll::Pending,
            },
        }
    }
}

#[cfg(test)]
mod async_event_tests {
    use super::*;
    use tokio_stream::StreamExt;

    #[tokio::test]
    async fn test_event_stream() {
        let (tx, rx) = broadcast::channel(2);

        let mut stream: EventStream<V2> = EventStream::new(rx);

        tx.send(Event::NewPeer(Peer::new(1, 1))).unwrap();
        tx.send(Event::NewPeer(Peer::new(1, 1))).unwrap();

        assert!(matches!(stream.next().await.unwrap(), Event::NewPeer(_)));
        assert!(matches!(stream.next().await.unwrap(), Event::NewPeer(_)));
    }
}
