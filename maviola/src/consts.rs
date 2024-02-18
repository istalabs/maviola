//! Common constants.

use std::time::Duration;

/// Default heartbeat timeout.
pub const DEFAULT_HEARTBEAT_TIMEOUT: Duration = Duration::from_millis(1200);
/// Default heartbeat interval.
pub const DEFAULT_HEARTBEAT_INTERVAL: Duration = Duration::from_millis(1000);

pub(crate) const UDP_RETRIES: usize = 5;
pub(crate) const UDP_RETRY_INTERVAL: Duration = Duration::from_millis(20);
