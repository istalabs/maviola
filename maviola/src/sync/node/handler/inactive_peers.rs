use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::{Duration, SystemTime};

use crate::core::io::ConnectionInfo;
use crate::core::utils::Closable;
use crate::protocol::Peer;
use crate::sync::node::Event;

use crate::prelude::*;

pub(in crate::sync::node) struct InactivePeersHandler<V: MaybeVersioned> {
    pub(in crate::sync::node) info: ConnectionInfo,
    pub(in crate::sync::node) peers: Arc<RwLock<HashMap<MavLinkId, Peer>>>,
    pub(in crate::sync::node) timeout: Duration,
    pub(in crate::sync::node) events_tx: mpmc::Sender<Event<V>>,
}

impl<V: MaybeVersioned + 'static> InactivePeersHandler<V> {
    pub(in crate::sync::node) fn spawn(self, state: Closable) {
        thread::spawn(move || {
            let info = &self.info;

            while !state.is_closed() {
                thread::sleep(self.timeout);
                let now = SystemTime::now();

                let inactive_peers = match self.peers.read() {
                    Ok(peers) => {
                        let mut inactive_peers = HashSet::new();
                        for peer in peers.values() {
                            if let Ok(since) = now.duration_since(peer.last_active) {
                                if since > self.timeout {
                                    inactive_peers.insert(peer.id);
                                }
                            }
                        }
                        inactive_peers
                    }
                    Err(err) => {
                        log::error!("[{info:?}] can't read peers: {err:?}");
                        break;
                    }
                };

                match self.peers.write() {
                    Ok(mut peers) => {
                        for id in inactive_peers {
                            if let Some(peer) = peers.remove(&id) {
                                if let Err(err) = self.events_tx.send(Event::PeerLost(peer)) {
                                    log::trace!(
                                        "[{info:?}] failed to report lost peer event: {err}"
                                    );
                                    break;
                                }
                            }
                        }
                    }
                    Err(err) => {
                        log::error!("[{info:?}] can't update peers: {err:?}");
                        break;
                    }
                }
            }

            if let Ok(mut peers) = self.peers.write() {
                for peer in peers.values() {
                    let _ = self.events_tx.send(Event::PeerLost(peer.clone()));
                }
                peers.clear();
            }

            log::trace!("[{info:?}] inactive peers handler stopped");
        });
    }
}
