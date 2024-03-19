use std::marker::PhantomData;
use std::time::Duration;

use crate::asnc::node::api::FrameSender;
use crate::core::io::ConnectionInfo;
use crate::core::utils::{make_heartbeat_message, Guarded, SharedCloser, Switch};
use crate::protocol::DialectVersion;

use crate::prelude::*;

pub(in crate::asnc::node) struct HeartbeatEmitter<V: Versioned> {
    pub(in crate::asnc::node) info: ConnectionInfo,
    pub(in crate::asnc::node) endpoint: Endpoint<V>,
    pub(in crate::asnc::node) interval: Duration,
    pub(in crate::asnc::node) sender: FrameSender<V>,
    pub(in crate::asnc::node) dialect_version: Option<DialectVersion>,
    pub(in crate::asnc::node) _version: PhantomData<V>,
}

impl<V: Versioned> HeartbeatEmitter<V> {
    pub(in crate::asnc::node) fn spawn(self, mut is_active: Guarded<SharedCloser, Switch>) {
        let heartbeat_message = make_heartbeat_message(self.dialect_version);

        tokio::spawn(async move {
            let info = &self.info;

            while is_active.is() {
                let frame = self.endpoint.next_frame(&heartbeat_message).unwrap();

                log::trace!("[{info:?}] broadcasting heartbeat");
                if let Err(err) = self.sender.send(&frame) {
                    log::trace!("[{info:?}] heartbeat can't be broadcast: {err:?}");
                    is_active.set(false);
                    break;
                }

                tokio::time::sleep(self.interval).await;
            }

            log::debug!("[{info:?}] heartbeats emitter stopped");
        });
    }
}
