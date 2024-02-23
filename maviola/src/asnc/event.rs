use crate::asnc::AsyncCallback;
use crate::protocol::Peer;

use crate::prelude::*;

/// Events.
#[derive(Clone, Debug)]
pub enum AsyncEvent<V: MaybeVersioned> {
    /// New [`Peer`] appeared in the network.
    NewPeer(Peer),
    /// A [`Peer`] was lost due to the timeout.
    PeerLost(Peer),
    /// New [`Frame`] received.
    Frame(Frame<V>, AsyncCallback<V>),
}
