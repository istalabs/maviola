//! # Common constants

use std::time::Duration;

mod default_dialect {
    #[cfg(feature = "all")]
    pub use crate::dialects::All as DefaultDialect;
    #[cfg(all(not(feature = "all"), feature = "ardupilotmega"))]
    pub use crate::dialects::Ardupilotmega as DefaultDialect;
    #[cfg(all(not(feature = "ardupilotmega"), feature = "common"))]
    pub use crate::dialects::Common as DefaultDialect;
    #[cfg(not(feature = "common"))]
    pub use crate::dialects::Minimal as DefaultDialect;
}

/// Default MAVLink dialect.
///
/// This dialect will be used as default by all Maviola entities and re-exported in
/// [`prelude`](crate::prelude).
///
/// The rules for determining the default dialect are the following order of dialect inclusion:
///
/// [`all`](https://mavlink.io/en/messages/all.html) >
/// [`ardupilotmega`](https://mavlink.io/en/messages/common.html) >
/// [`common`](https://mavlink.io/en/messages/common.html) >
/// [`minimal`]((https://mavlink.io/en/messages/minimal.html))
///
/// That means, that if you enabled `ardupilotmega` dialect but not `all`, then the former is the
/// "greatest" dialect that include others, and it will be chosen as a default dialect.
///
/// ---
#[doc(inline)]
pub use default_dialect::DefaultDialect;

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
