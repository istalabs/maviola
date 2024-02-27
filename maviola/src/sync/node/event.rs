use crate::protocol::Peer;
use crate::sync::io::Callback;

use crate::prelude::*;

/// <sup>[`sync`](crate::sync)</sup>
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

pub(crate) struct EventsIterator<V: MaybeVersioned + 'static> {
    pub(crate) rx: mpmc::Receiver<Event<V>>,
}

impl<V: MaybeVersioned> Iterator for EventsIterator<V> {
    type Item = Event<V>;

    fn next(&mut self) -> Option<Self::Item> {
        self.rx.recv().ok()
    }
}
