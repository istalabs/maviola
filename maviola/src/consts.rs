//! Common constants.

use std::time::Duration;

/// Default heartbeat timeout.
pub const DEFAULT_HEARTBEAT_TIMEOUT: Duration = Duration::from_millis(1000);
/// Heartbeat timeout tolerance.
pub const HEARTBEAT_TIMEOUT_TOLERANCE: f64 = 1.5;
