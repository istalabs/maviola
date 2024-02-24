use std::marker::PhantomData;
use std::sync::atomic::AtomicU8;
use std::sync::{atomic, Arc};
use std::thread;
use std::time::Duration;

use crate::core::io::ConnectionInfo;
use crate::core::marker::Identified;
use crate::core::utils::{make_heartbeat_message, Guarded, SharedCloser, Switch};
use crate::sync::conn::ConnSender;

use crate::prelude::*;

pub(in crate::sync::node) struct HeartbeatEmitter<D: Dialect, V: Versioned + 'static> {
    pub(in crate::sync::node) info: ConnectionInfo,
    pub(in crate::sync::node) id: Identified,
    pub(in crate::sync::node) interval: Duration,
    pub(in crate::sync::node) sender: ConnSender<V>,
    pub(in crate::sync::node) sequence: Arc<AtomicU8>,
    pub(in crate::sync::node) _version: PhantomData<V>,
    pub(in crate::sync::node) _dialect: PhantomData<D>,
}

impl<D: Dialect, V: Versioned + 'static> HeartbeatEmitter<D, V> {
    pub(in crate::sync::node) fn spawn(self, mut is_active: Guarded<SharedCloser, Switch>) {
        let heartbeat_message = make_heartbeat_message::<D>();

        thread::spawn(move || {
            let info = &self.info;

            while is_active.is() {
                let sequence = self.sequence.fetch_add(1, atomic::Ordering::Relaxed);
                let frame = Frame::builder()
                    .sequence(sequence)
                    .system_id(self.id.system_id)
                    .component_id(self.id.component_id)
                    .version(V::v())
                    .message(&heartbeat_message)
                    .unwrap()
                    .build();

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
