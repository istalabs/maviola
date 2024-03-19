//! # Common constants

use std::time::Duration;

/// Default heartbeat timeout.
pub const DEFAULT_HEARTBEAT_TIMEOUT: Duration = Duration::from_millis(1200);
/// Default heartbeat interval.
pub const DEFAULT_HEARTBEAT_INTERVAL: Duration = Duration::from_millis(1000);
/// Default host for client to bind to.
pub const DEFAULT_UDP_HOST: &str = "127.0.0.1";

/// Time out after which it is guaranteed, that server connection will initiate closing procedure.
pub const SERVER_HANG_UP_TIMEOUT: Duration = Duration::from_millis(50);

/// Specifies pooling interval for node's incoming frame handler.
pub(crate) const INCOMING_FRAMES_POOLING_INTERVAL: Duration = Duration::from_micros(50);

/// Specifies a pooling interval for network nodes.
pub(crate) const NETWORK_POOLING_INTERVAL: Duration = Duration::from_micros(50);
