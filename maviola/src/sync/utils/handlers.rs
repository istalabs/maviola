use std::thread;

use crate::core::io::ConnectionInfo;
use crate::core::utils::Closer;
use crate::sync::consts::CONN_STOP_POOLING_INTERVAL;

pub(crate) fn handle_listener_stop(
    handler: thread::JoinHandle<crate::core::error::Result<Closer>>,
    info: ConnectionInfo,
) {
    thread::spawn(move || {
        while !handler.is_finished() {
            thread::sleep(CONN_STOP_POOLING_INTERVAL);
        }

        match handler.join() {
            Ok(res) => match res {
                Ok(closer) => {
                    closer.close();
                    log::debug!("[{info:?}] listener stopped")
                }
                Err(err) => {
                    log::debug!("[{info:?}] listener exited with error: {err:?}")
                }
            },
            Err(err) => {
                log::error!("[{info:?}] listener failed: {err:?}");
            }
        }
    });
}
