use std::marker::PhantomData;
use std::thread;
use std::time::Duration;

use crate::core::io::ConnectionInfo;
use crate::core::utils::{make_heartbeat_message, Guarded, SharedCloser, Switch};
use crate::sync::node::api::FrameSender;

use crate::prelude::*;

pub(in crate::sync::node) struct HeartbeatEmitter<D: Dialect, V: Versioned + 'static> {
    pub(in crate::sync::node) info: ConnectionInfo,
    pub(in crate::sync::node) endpoint: Endpoint<V>,
    pub(in crate::sync::node) interval: Duration,
    pub(in crate::sync::node) sender: FrameSender<V>,
    pub(in crate::sync::node) _version: PhantomData<V>,
    pub(in crate::sync::node) _dialect: PhantomData<D>,
}

impl<D: Dialect, V: Versioned + 'static> HeartbeatEmitter<D, V> {
    pub(in crate::sync::node) fn spawn(self, mut is_active: Guarded<SharedCloser, Switch>) {
        let heartbeat_message = make_heartbeat_message::<D>();

        thread::spawn(move || {
            let info = &self.info;

            while is_active.is() {
                let frame = self.endpoint.next_frame(&heartbeat_message).unwrap();

                log::trace!("[{info:?}] broadcasting heartbeat");
                if let Err(err) = self.sender.send(&frame) {
                    log::trace!("[{info:?}] heartbeat can't be broadcast: {err:?}");
                    is_active.set(false);
                    break;
                }

                thread::sleep(self.interval);
            }

            log::debug!("[{info:?}] heartbeats emitter stopped");
        });
    }
}
