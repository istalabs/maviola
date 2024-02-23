use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::{Duration, SystemTime};

use crate::core::io::ConnectionInfo;
use crate::core::utils::Closable;
use crate::protocol::{Peer, PeerId};
use crate::sync::Event;

use crate::prelude::*;

pub(crate) struct InactivePeersHandler<'a, V: MaybeVersioned> {
    pub(crate) info: &'a ConnectionInfo,
    pub(crate) peers: Arc<RwLock<HashMap<PeerId, Peer>>>,
    pub(crate) timeout: Duration,
    pub(crate) events_tx: mpmc::Sender<Event<V>>,
}

impl<V: MaybeVersioned + 'static> InactivePeersHandler<'_, V> {
    pub(crate) fn spawn(self, state: Closable) {
        let info = self.info.clone();
        let peers = self.peers.clone();
        let heartbeat_timeout = self.timeout;
        let events_tx = self.events_tx.clone();

        thread::spawn(move || {
            loop {
                if state.is_closed() {
                    log::trace!("[{info:?}] closing inactive peers handler: node is disconnected");
                    break;
                }

                thread::sleep(heartbeat_timeout);
                let now = SystemTime::now();

                let inactive_peers = match peers.read() {
                    Ok(peers) => {
                        let mut inactive_peers = HashSet::new();
                        for peer in peers.values() {
                            if let Ok(since) = now.duration_since(peer.last_active) {
                                if since > heartbeat_timeout {
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

                match peers.write() {
                    Ok(mut peers) => {
                        for id in inactive_peers {
                            if let Some(peer) = peers.remove(&id) {
                                if let Err(err) = events_tx.send(Event::PeerLost(peer)) {
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

            log::trace!("[{info:?}] inactive peers handler stopped");
        });
    }
}
