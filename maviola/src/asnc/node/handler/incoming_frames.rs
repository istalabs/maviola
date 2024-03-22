use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::asnc::io::IncomingFrameReceiver;
use crate::asnc::node::api::EventSender;
use crate::core::consts::INCOMING_FRAMES_POOLING_INTERVAL;
use crate::core::io::ConnectionInfo;
use crate::core::marker::Proxy;
use crate::core::utils::Closable;
use crate::dialects::Minimal;
use crate::error::RecvTimeoutError;
use crate::protocol::Peer;

use crate::asnc::prelude::*;
use crate::prelude::*;

pub(in crate::asnc::node) struct IncomingFramesHandler<V: MaybeVersioned> {
    pub(in crate::asnc::node) info: ConnectionInfo,
    pub(in crate::asnc::node) peers: Arc<RwLock<HashMap<MavLinkId, Peer>>>,
    pub(in crate::asnc::node) receiver: IncomingFrameReceiver<V>,
    pub(in crate::asnc::node) event_sender: EventSender<V>,
    pub(in crate::asnc::node) sender: FrameSender<V, Proxy>,
}

impl<V: MaybeVersioned> IncomingFramesHandler<V> {
    pub(in crate::asnc::node) fn spawn(mut self, state: Closable) {
        tokio::spawn(async move {
            let info = &self.info;

            while !state.is_closed() {
                let (frame, callback) = match self
                    .receiver
                    .recv_timeout(INCOMING_FRAMES_POOLING_INTERVAL)
                    .await
                {
                    Ok(frame) => {
                        let (frame, channel) = frame.into();
                        let callback = Callback::new(channel, self.sender.clone());
                        (frame, callback)
                    }
                    Err(err) => match err {
                        RecvTimeoutError::Disconnected => {
                            break;
                        }
                        _ => continue,
                    },
                };

                if let Ok(Minimal::Heartbeat(_)) = frame.decode() {
                    let peer = Peer::new(frame.system_id(), frame.component_id());
                    log::trace!("[{info:?}] received heartbeat from {peer:?}");

                    if self.handle_new_peer(peer).await.is_err() {
                        break;
                    }
                }

                if self.handle_incoming_frame(frame, callback).is_err() {
                    break;
                }
            }

            log::trace!("[{info:?}] incoming frames handler stopped");
        });
    }

    async fn handle_new_peer(&self, peer: Peer) -> Result<()> {
        let mut peers = self.peers.write().await;

        let has_peer = peers.contains_key(&peer.id);
        peers.insert(peer.id, peer.clone());

        if !has_peer {
            if let Err(err) = self.event_sender.send(Event::NewPeer(peer)) {
                log::trace!(
                    "[{:?}] failed to report new peer event: {err:?}",
                    &self.info
                );
                return Err(Error::from(err));
            }
        }

        Ok(())
    }

    fn handle_incoming_frame(&self, frame: Frame<V>, callback: Callback<V>) -> Result<()> {
        let event_send_result = self
            .event_sender
            .send(Event::Frame(frame.clone(), callback));

        if let Err(err) = event_send_result {
            log::trace!(
                "[{:?}] failed to report incoming frame event: {err:?}",
                &self.info
            );
            return Err(Error::from(err));
        }

        Ok(())
    }
}
