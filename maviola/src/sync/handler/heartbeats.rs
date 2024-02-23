use std::marker::PhantomData;
use std::sync::atomic::AtomicU8;
use std::sync::{atomic, Arc};
use std::thread;
use std::time::Duration;

use crate::core::io::ConnectionInfo;
use crate::core::marker::Identified;
use crate::core::utils::{Guarded, SharedCloser, Switch};
use crate::sync::conn::ConnSender;

use crate::prelude::*;

pub(crate) struct HeartbeatEmitter<D: Dialect, V: Versioned + 'static> {
    pub(crate) info: ConnectionInfo,
    pub(crate) id: Identified,
    pub(crate) interval: Duration,
    pub(crate) version: V,
    pub(crate) sender: ConnSender<V>,
    pub(crate) sequence: Arc<AtomicU8>,
    pub(crate) _dialect: PhantomData<D>,
}

impl<D: Dialect, V: Versioned + 'static> HeartbeatEmitter<D, V> {
    pub(crate) fn spawn(self, mut is_active: Guarded<SharedCloser, Switch>) {
        let heartbeat_message = self.make_heartbeat_message();

        thread::spawn(move || {
            let info = &self.info;

            loop {
                if !is_active.is() {
                    log::trace!(
                        "[{info:?}] closing heartbeat emitter since node is no longer active"
                    );
                    break;
                }

                let sequence = self.sequence.fetch_add(1, atomic::Ordering::Relaxed);
                let frame = Frame::builder()
                    .sequence(sequence)
                    .system_id(self.id.system_id)
                    .component_id(self.id.component_id)
                    .version(self.version.clone())
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

    fn make_heartbeat_message(&self) -> mavio::dialects::minimal::messages::Heartbeat {
        use crate::dialects::minimal as dialect;

        dialect::messages::Heartbeat {
            type_: Default::default(),
            autopilot: dialect::enums::MavAutopilot::Generic,
            base_mode: Default::default(),
            custom_mode: 0,
            system_status: dialect::enums::MavState::Active,
            mavlink_version: D::version().unwrap_or_default(),
        }
    }
}
