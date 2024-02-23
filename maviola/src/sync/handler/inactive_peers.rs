use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::{Duration, SystemTime};

use crate::core::io::ConnectionInfo;
use crate::core::utils::Closable;
use crate::protocol::{Peer, PeerId};
use crate::sync::Event;

use crate::prelude::*;

pub(crate) struct InactivePeersHandler<V: MaybeVersioned> {
    pub(crate) info: ConnectionInfo,
    pub(crate) peers: Arc<RwLock<HashMap<PeerId, Peer>>>,
    pub(crate) timeout: Duration,
    pub(crate) events_tx: mpmc::Sender<Event<V>>,
}

impl<V: MaybeVersioned + 'static> InactivePeersHandler<V> {
    pub(crate) fn spawn(self, state: Closable) {
        thread::spawn(move || {
            let info = &self.info;

            loop {
                if state.is_closed() {
                    log::trace!("[{info:?}] closing inactive peers handler: node is disconnected");
                    break;
                }

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

            log::trace!("[{info:?}] inactive peers handler stopped");
        });
    }
}
