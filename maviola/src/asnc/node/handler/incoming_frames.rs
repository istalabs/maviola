use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::asnc::conn::AsyncConnReceiver;
use crate::asnc::AsyncEvent;
use crate::core::io::ConnectionInfo;
use crate::core::utils::Closable;
use crate::protocol::{Peer, PeerId};

use crate::prelude::*;

pub(in crate::asnc::node) struct IncomingFramesHandler<V: MaybeVersioned + 'static> {
    pub(in crate::asnc::node) info: ConnectionInfo,
    pub(in crate::asnc::node) peers: Arc<RwLock<HashMap<PeerId, Peer>>>,
    pub(in crate::asnc::node) receiver: AsyncConnReceiver<V>,
    pub(in crate::asnc::node) events_tx: broadcast::Sender<AsyncEvent<V>>,
}

impl<V: MaybeVersioned + 'static> IncomingFramesHandler<V> {
    pub(in crate::asnc::node) async fn spawn(mut self, state: Closable) {
        tokio::spawn(async move {
            let info = &self.info;

            while !state.is_closed() {
                let (frame, response) = match self.receiver.recv().await {
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

                    {
                        let mut peers = self.peers.write().await;
                        let has_peer = peers.contains_key(&peer.id);
                        peers.insert(peer.id, peer.clone());

                        if !has_peer {
                            if let Err(err) = self.events_tx.send(AsyncEvent::NewPeer(peer)) {
                                log::trace!("[{info:?}] failed to report new peer event: {err}");
                                return;
                            }
                        }
                    }
                }

                if let Err(err) = self
                    .events_tx
                    .send(AsyncEvent::Frame(frame.clone(), response))
                {
                    log::trace!("[{info:?}] failed to report incoming frame event: {err}");
                    return;
                }
            }
        });
    }
}
