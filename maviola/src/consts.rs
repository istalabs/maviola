//! # Common constants

use std::time::Duration;

/// Default heartbeat timeout.
pub const DEFAULT_HEARTBEAT_TIMEOUT: Duration = Duration::from_millis(1200);
/// Default heartbeat interval.
pub const DEFAULT_HEARTBEAT_INTERVAL: Duration = Duration::from_millis(1000);
/// Default host for client to bind to.
pub const DEFAULT_UDP_HOST: &str = "127.0.0.1";
