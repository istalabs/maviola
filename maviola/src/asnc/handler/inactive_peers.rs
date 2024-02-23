use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use tokio::sync::RwLock;

use crate::asnc::AsyncEvent;
use crate::core::io::ConnectionInfo;
use crate::core::utils::Closable;
use crate::protocol::{Peer, PeerId};

use crate::prelude::*;

pub(crate) struct InactivePeersHandler<V: MaybeVersioned> {
    pub(crate) info: ConnectionInfo,
    pub(crate) peers: Arc<RwLock<HashMap<PeerId, Peer>>>,
    pub(crate) timeout: Duration,
    pub(crate) events_tx: broadcast::Sender<AsyncEvent<V>>,
}

impl<V: MaybeVersioned + 'static> InactivePeersHandler<V> {
    pub(crate) async fn spawn(self, state: Closable) {
        tokio::spawn(async move {
            let info = &self.info;

            while !state.is_closed() {
                tokio::time::sleep(self.timeout).await;
                let now = SystemTime::now();

                let inactive_peers = {
                    let peers = self.peers.read().await;
                    let mut inactive_peers = HashSet::new();
                    for peer in peers.values() {
                        if let Ok(since) = now.duration_since(peer.last_active) {
                            if since > self.timeout {
                                inactive_peers.insert(peer.id);
                            }
                        }
                    }
                    inactive_peers
                };

                {
                    let mut peers = self.peers.write().await;

                    for id in inactive_peers {
                        if let Some(peer) = peers.remove(&id) {
                            if let Err(err) = self.events_tx.send(AsyncEvent::PeerLost(peer)) {
                                log::trace!("[{info:?}] failed to report lost peer event: {err}");
                                break;
                            }
                        }
                    }
                }
            }

            log::trace!("[{info:?}] inactive peers handler stopped");
        });
    }
}
