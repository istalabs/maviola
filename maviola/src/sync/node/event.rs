use std::thread;

use crate::core::utils::Closable;
use crate::error::{FrameError, TryRecvError};
use crate::protocol::Peer;
use crate::sync::consts::EVENTS_RECV_POOLING_INTERVAL;
use crate::sync::node::api::EventReceiver;
use crate::sync::node::Callback;

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
    /// New [`Frame`] received, but it hasn't passed validation.
    Invalid(Frame<V>, FrameError, Callback<V>),
}

pub(crate) struct EventsIterator<V: MaybeVersioned> {
    receiver: EventReceiver<V>,
    state: Closable,
}

impl<V: MaybeVersioned> EventsIterator<V> {
    pub fn new(receiver: EventReceiver<V>, state: Closable) -> Self {
        Self { receiver, state }
    }
}

impl<V: MaybeVersioned> Iterator for EventsIterator<V> {
    type Item = Event<V>;

    fn next(&mut self) -> Option<Self::Item> {
        while !self.state.is_closed() {
            return match self.receiver.try_recv() {
                Ok(event) => Some(event),
                Err(err) => match err {
                    TryRecvError::Disconnected => None,
                    _ => {
                        thread::sleep(EVENTS_RECV_POOLING_INTERVAL);
                        continue;
                    }
                },
            };
        }
        self.receiver.try_recv().ok()
    }
}
