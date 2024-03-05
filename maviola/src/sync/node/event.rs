use std::sync::mpsc::TryRecvError;
use std::thread;

use crate::core::utils::Closable;
use crate::protocol::Peer;
use crate::sync::consts::EVENTS_RECV_POOLING_INTERVAL;
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
    rx: mpmc::Receiver<Event<V>>,
    state: Closable,
}

impl<V: MaybeVersioned + 'static> EventsIterator<V> {
    pub fn new(rx: mpmc::Receiver<Event<V>>, state: Closable) -> Self {
        Self { rx, state }
    }
}

impl<V: MaybeVersioned> Iterator for EventsIterator<V> {
    type Item = Event<V>;

    fn next(&mut self) -> Option<Self::Item> {
        while !self.state.is_closed() {
            return match self.rx.try_recv() {
                Ok(event) => Some(event),
                Err(err) => match err {
                    TryRecvError::Empty => {
                        thread::sleep(EVENTS_RECV_POOLING_INTERVAL);
                        continue;
                    }
                    TryRecvError::Disconnected => None,
                },
            };
        }
        self.rx.try_recv().ok()
    }
}
