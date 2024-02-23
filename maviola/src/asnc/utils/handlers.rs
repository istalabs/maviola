use crate::asnc::consts::CONN_STOP_POOLING_INTERVAL;
use crate::core::io::ConnectionInfo;
use crate::core::utils::Closer;
use std::time::Duration;

pub(crate) fn handle_listener_stop(
    handler: tokio::task::JoinHandle<crate::core::error::Result<Closer>>,
    info: ConnectionInfo,
) {
    tokio::task::spawn(async move {
        match handler.await {
            Ok(res) => match res {
                Ok(mut closer) => {
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
