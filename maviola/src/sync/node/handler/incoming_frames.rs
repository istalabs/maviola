use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::thread;

use crate::core::io::ConnectionInfo;
use crate::core::utils::Closable;
use crate::protocol::{Peer, PeerId};
use crate::sync::conn::ConnReceiver;
use crate::sync::Event;

use crate::prelude::*;

pub(in crate::sync::node) struct IncomingFramesHandler<V: MaybeVersioned + 'static> {
    pub(in crate::sync::node) info: ConnectionInfo,
    pub(in crate::sync::node) peers: Arc<RwLock<HashMap<PeerId, Peer>>>,
    pub(in crate::sync::node) receiver: ConnReceiver<V>,
    pub(in crate::sync::node) events_tx: mpmc::Sender<Event<V>>,
}

impl<V: MaybeVersioned + 'static> IncomingFramesHandler<V> {
    pub(in crate::sync::node) fn spawn(self, state: Closable) {
        thread::spawn(move || {
            let info = &self.info;

            while !state.is_closed() {
                let (frame, response) = match self.receiver.try_recv() {
                    Ok((frame, resp)) => (frame, resp),
                    Err(Error::Sync(err)) => match err {
                        SyncError::Empty => continue,
                        _ => {
                            log::trace!("[{info:?}] node connection closed");
                            return;
                        }
                    },
                    Err(err) => {
                        log::error!("[{info:?}] unhandled node error: {err}");
                        return;
                    }
                };

                if let Ok(Minimal::Heartbeat(_)) = frame.decode() {
                    let peer = Peer::new(frame.system_id(), frame.component_id());
                    log::trace!("[{info:?}] received heartbeat from {peer:?}");

                    match self.peers.write() {
                        Ok(mut peers) => {
                            let has_peer = peers.contains_key(&peer.id);
                            peers.insert(peer.id, peer.clone());

                            if !has_peer {
                                if let Err(err) = self.events_tx.send(Event::NewPeer(peer)) {
                                    log::trace!(
                                        "[{info:?}] failed to report new peer event: {err}"
                                    );
                                    return;
                                }
                            }
                        }
                        Err(err) => {
                            log::trace!(
                                "[{info:?}] received {peer:?} but node is offline: {err:?}"
                            );
                            return;
                        }
                    }
                }

                if let Err(err) = self.events_tx.send(Event::Frame(frame.clone(), response)) {
                    log::trace!("[{info:?}] failed to report incoming frame event: {err}");
                    return;
                }
            }
        });
    }
}
