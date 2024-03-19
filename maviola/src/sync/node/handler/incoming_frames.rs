use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::thread;

use crate::core::consts::INCOMING_FRAMES_POOLING_INTERVAL;
use crate::core::error::RecvTimeoutError;
use crate::core::io::ConnectionInfo;
use crate::core::utils::Closable;
use crate::protocol::Peer;
use crate::sync::io::IncomingFrameReceiver;
use crate::sync::node::api::{EventSender, FrameSender};
use crate::sync::node::{Callback, Event};

use crate::prelude::*;

pub(in crate::sync::node) struct IncomingFramesHandler<V: MaybeVersioned> {
    pub(in crate::sync::node) info: ConnectionInfo,
    pub(in crate::sync::node) peers: Arc<RwLock<HashMap<MavLinkId, Peer>>>,
    pub(in crate::sync::node) receiver: IncomingFrameReceiver<V>,
    pub(in crate::sync::node) event_sender: EventSender<V>,
    pub(in crate::sync::node) sender: FrameSender<V>,
}

impl<V: MaybeVersioned> IncomingFramesHandler<V> {
    pub(in crate::sync::node) fn spawn(self, state: Closable) {
        thread::spawn(move || {
            let info = &self.info;

            while !state.is_closed() {
                let (frame, callback) =
                    match self.receiver.recv_timeout(INCOMING_FRAMES_POOLING_INTERVAL) {
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

                    if self.handle_new_peer(peer).is_err() {
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

    fn handle_new_peer(&self, peer: Peer) -> Result<()> {
        let info = &self.info;

        match self.peers.write() {
            Ok(mut peers) => {
                let has_peer = peers.contains_key(&peer.id);
                peers.insert(peer.id, peer.clone());

                if !has_peer {
                    if let Err(err) = self.event_sender.send(Event::NewPeer(peer)) {
                        log::trace!("[{info:?}] failed to report new peer: {err:?}");
                        return Err(Error::from(err));
                    }
                }
            }
            Err(err) => {
                log::trace!("[{info:?}] received {peer:?}, but node is offline: {err:?}");
                return Err(Error::from(err));
            }
        }

        Ok(())
    }

    fn handle_incoming_frame(&self, frame: Frame<V>, callback: Callback<V>) -> Result<()> {
        let event_send_result = self.event_sender.send(Event::Frame(frame, callback));

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
