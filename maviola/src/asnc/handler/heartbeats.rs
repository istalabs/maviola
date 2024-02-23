use std::marker::PhantomData;
use std::sync::atomic::AtomicU8;
use std::sync::{atomic, Arc};
use std::time::Duration;

use crate::asnc::conn::AsyncConnSender;
use crate::core::io::ConnectionInfo;
use crate::core::marker::Identified;
use crate::core::utils::{make_heartbeat_message, Guarded, SharedCloser, Switch};

use crate::prelude::*;

pub(crate) struct HeartbeatEmitter<D: Dialect, V: Versioned + 'static> {
    pub(crate) info: ConnectionInfo,
    pub(crate) id: Identified,
    pub(crate) interval: Duration,
    pub(crate) version: V,
    pub(crate) sender: AsyncConnSender<V>,
    pub(crate) sequence: Arc<AtomicU8>,
    pub(crate) _dialect: PhantomData<D>,
}

impl<D: Dialect, V: Versioned + 'static> HeartbeatEmitter<D, V> {
    pub(crate) async fn spawn(self, mut is_active: Guarded<SharedCloser, Switch>) {
        let heartbeat_message = make_heartbeat_message::<D>();

        tokio::spawn(async move {
            let info = &self.info;

            while is_active.is() {
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

                tokio::time::sleep(self.interval).await;
            }
            log::debug!("[{info:?}] heartbeats emitter stopped");
        });
    }
}
