use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use tokio::sync::RwLock;

use crate::asnc::node::AsyncEvent;
use crate::core::io::ConnectionInfo;
use crate::core::utils::Closable;
use crate::protocol::Peer;

use crate::prelude::*;

pub(in crate::asnc::node) struct InactivePeersHandler<V: MaybeVersioned> {
    pub(in crate::asnc::node) info: ConnectionInfo,
    pub(in crate::asnc::node) peers: Arc<RwLock<HashMap<MavLinkId, Peer>>>,
    pub(in crate::asnc::node) timeout: Duration,
    pub(in crate::asnc::node) events_tx: broadcast::Sender<AsyncEvent<V>>,
}

impl<V: MaybeVersioned + 'static> InactivePeersHandler<V> {
    pub(in crate::asnc::node) async fn spawn(self, state: Closable) {
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
