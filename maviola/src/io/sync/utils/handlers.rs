use std::thread;

use crate::io::sync::consts::CONN_STOP_POOLING_INTERVAL;
use crate::io::ConnectionInfo;
use crate::utils::Closer;

pub(crate) fn handle_listener_stop(
    handler: thread::JoinHandle<crate::errors::Result<Closer>>,
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
