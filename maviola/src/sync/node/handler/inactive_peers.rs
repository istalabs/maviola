use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::{Duration, SystemTime};

use crate::core::io::ConnectionInfo;
use crate::core::utils::Closable;
use crate::protocol::Peer;
use crate::sync::node::api::EventSender;
use crate::sync::node::Event;

use crate::prelude::*;

pub(in crate::sync::node) struct InactivePeersHandler<V: MaybeVersioned> {
    pub(in crate::sync::node) info: ConnectionInfo,
    pub(in crate::sync::node) peers: Arc<RwLock<HashMap<MavLinkId, Peer>>>,
    pub(in crate::sync::node) timeout: Duration,
    pub(in crate::sync::node) event_sender: EventSender<V>,
}

impl<V: MaybeVersioned + 'static> InactivePeersHandler<V> {
    pub(in crate::sync::node) fn spawn(self, state: Closable) {
        thread::spawn(move || {
            while !state.is_closed() {
                thread::sleep(self.timeout);

                let inactive_peers = match self.collect_inactive_peers() {
                    Ok(inactive_peers) => inactive_peers,
                    Err(_) => break,
                };

                if self.handle_inactive_peers(inactive_peers).is_err() {
                    break;
                }
            }

            self.shutdown();
        });
    }

    fn collect_inactive_peers(&self) -> Result<HashSet<MavLinkId>> {
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
                log::error!("[{:?}] can't read peers: {err:?}", self.info);
                return Err(Error::from(err));
            }
        };

        Ok(inactive_peers)
    }

    fn handle_inactive_peers(&self, inactive_peers: HashSet<MavLinkId>) -> Result<()> {
        let info = &self.info;

        match self.peers.write() {
            Ok(mut peers) => {
                for id in inactive_peers {
                    if let Some(peer) = peers.remove(&id) {
                        if let Err(err) = self.event_sender.send(Event::PeerLost(peer)) {
                            log::trace!("[{info:?}] failed to report lost peer event: {err:?}");
                            return Err(Error::from(err));
                        }
                    }
                }
            }
            Err(err) => {
                log::error!("[{info:?}] can't update peers: {err:?}");
                return Err(Error::from(err));
            }
        }

        Ok(())
    }

    fn shutdown(&self) {
        if let Ok(mut peers) = self.peers.write() {
            for peer in peers.values() {
                let _ = self.event_sender.send(Event::PeerLost(peer.clone()));
            }
            peers.clear();
        }
        log::trace!("[{:?}] inactive peers handler stopped", self.info);
    }
}
