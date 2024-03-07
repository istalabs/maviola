use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use crate::asnc::node::api::EventSender;
use tokio::sync::RwLock;

use crate::asnc::node::Event;
use crate::core::io::ConnectionInfo;
use crate::core::utils::Closable;
use crate::protocol::Peer;

use crate::prelude::*;

pub(in crate::asnc::node) struct InactivePeersHandler<V: MaybeVersioned> {
    pub(in crate::asnc::node) info: ConnectionInfo,
    pub(in crate::asnc::node) peers: Arc<RwLock<HashMap<MavLinkId, Peer>>>,
    pub(in crate::asnc::node) timeout: Duration,
    pub(in crate::asnc::node) event_sender: EventSender<V>,
}

impl<V: MaybeVersioned + 'static> InactivePeersHandler<V> {
    pub(in crate::asnc::node) fn spawn(self, state: Closable) {
        tokio::spawn(async move {
            while !state.is_closed() {
                tokio::time::sleep(self.timeout).await;

                let inactive_peers = self.collect_inactive_peers().await;

                if self.handle_inactive_peers(inactive_peers).await.is_err() {
                    break;
                }
            }

            self.shutdown().await;
        });
    }

    async fn collect_inactive_peers(&self) -> HashSet<MavLinkId> {
        let now = SystemTime::now();

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
    }

    pub async fn handle_inactive_peers(&self, inactive_peers: HashSet<MavLinkId>) -> Result<()> {
        let mut peers = self.peers.write().await;

        for id in inactive_peers {
            if let Some(peer) = peers.remove(&id) {
                if let Err(err) = self.event_sender.send(Event::PeerLost(peer)) {
                    log::trace!(
                        "[{:?}] failed to report lost peer event: {err:?}",
                        &self.info
                    );
                    return Err(Error::from(err));
                }
            }
        }

        Ok(())
    }

    async fn shutdown(&self) {
        let mut peers = self.peers.write().await;

        for peer in peers.values() {
            let _ = self.event_sender.send(Event::PeerLost(peer.clone()));
        }
        peers.clear();

        log::trace!("[{:?}] inactive peers handler stopped", self.info);
    }
}
