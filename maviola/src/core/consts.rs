//! # Common constants

use std::time::Duration;

mod _default_dialect {
    #[cfg(feature = "all")]
    pub use crate::dialects::All as DefaultDialect;

    #[cfg(all(not(feature = "all"), feature = "ardupilotmega"))]
    pub use crate::dialects::Ardupilotmega as DefaultDialect;

    #[cfg(all(not(feature = "ardupilotmega"), feature = "common"))]
    pub use crate::dialects::Common as DefaultDialect;

    #[cfg(all(not(feature = "common"), feature = "standard"))]
    pub use crate::dialects::Standard as DefaultDialect;

    #[cfg(not(feature = "standard"))]
    pub use crate::dialects::Minimal as DefaultDialect;

    #[cfg(feature = "all")]
    pub use crate::dialects::all as default_dialect;

    #[cfg(all(not(feature = "all"), feature = "ardupilotmega"))]
    pub use crate::dialects::ardupilotmega as default_dialect;

    #[cfg(all(not(feature = "ardupilotmega"), feature = "common"))]
    pub use crate::dialects::common as default_dialect;

    #[cfg(all(not(feature = "common"), feature = "standard"))]
    pub use crate::dialects::standard as default_dialect;

    #[cfg(not(feature = "standard"))]
    pub use crate::dialects::minimal as default_dialect;
}

/// Default MAVLink dialect.
///
/// This dialect will be used as default by all Maviola entities and re-exported in
/// [`prelude`](crate::prelude).
///
/// The rules for determining the default dialect are defined by the following order of canonical dialect inclusion:
///
/// [`all`](https://mavlink.io/en/messages/all.html) >
/// [`ardupilotmega`](https://mavlink.io/en/messages/common.html) >
/// [`common`](https://mavlink.io/en/messages/common.html) >
/// [`standard`]((https://mavlink.io/en/messages/standard.html))
/// [`minimal`]((https://mavlink.io/en/messages/minimal.html))
///
/// That means, that if you enabled `ardupilotmega` dialect but not `all`, then the former is the
/// most general canonical dialect, and it will be chosen as a default one.
///
/// **⚠** Minimal dialect will be set as default even if `minimal` cargo feature is not enabled as
/// this dialect is required by Maviola internals.
///
/// ---
#[doc(inline)]
pub use _default_dialect::DefaultDialect;

/// Default MAVLink dialect module.
///
/// Similar to [`DefaultDialect`] but provides access to a dialect module instead of dialect itself.
/// Re-exported by [`prelude`](crate::prelude).
///
/// See [`DefaultDialect`] to learn about logic behind choosing a default dialect.
///
/// # Usage
///
/// ```rust,no_run
/// use maviola::prelude::default_dialect;
///
/// let message = default_dialect::messages::Heartbeat::default();
/// ```
///
/// ---
#[doc(inline)]
pub use _default_dialect::default_dialect;

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
