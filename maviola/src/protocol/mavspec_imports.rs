/// <sup>[`mavspec`](https://crates.io/crates/mavspec)</sup>
/// [MAVSpec](https://crates.io/crates/mavspec) re-exported
///
/// We re-export MAVSpec in order to simplify interoperability with the tools provided by this
/// library.
///
/// For example, [`derive`](mod@derive) proc macros depends on [`mavspec::rust::spec`] being
/// accessible.
///
/// ---
#[doc(inline)]
pub use mavio::mavspec;

/// <sup>[`mavspec`](https://crates.io/crates/mavspec)</sup>
/// MAVLink dialects
///
/// These dialects are generated by [MAVSpec](https://crates.io/crates/mavspec).
///
/// Each dialect belongs to a specific module, such as:
///
/// - [`minimal`](crate::protocol::dialects::minimal)
/// - [`common`](crate::protocol::dialects::common)
/// - [`ardupilotmega`](crate::protocol::dialects::ardupilotmega)
/// - ... and so on
///
/// Re-exported from [`mavspec::rust::dialects`].
///
/// ---
#[doc(inline)]
pub use mavspec::rust::dialects;

/// <sup>[`mavspec`](https://crates.io/crates/mavspec)</sup>
/// Default MAVLink dialect module
///
/// Similar to [`DefaultDialect`] but provides access to a dialect module instead of dialect itself.
///
/// See [`DefaultDialect`] to learn about logic behind choosing a default dialect.
///
/// # Usage
///
/// ```rust,no_run
/// use maviola::protocol::default_dialect;
///
/// let message = default_dialect::messages::Heartbeat::default();
/// ```
///
/// Requires at least `dlct-minimal` dialect feature flag to be enabled.
///
/// Re-exported from [`mavspec::rust::default_dialect`].
///
/// ---
#[doc(inline)]
pub use mavspec::rust::default_dialect;

/// <sup>[`mavspec`](https://crates.io/crates/mavspec)</sup>
/// Default MAVLink dialect
///
/// The rules for determining the default dialect are defined by the following order of canonical dialect inclusion:
///
/// [`all`](https://mavlink.io/en/messages/all.html) >
/// [`ardupilotmega`](https://mavlink.io/en/messages/common.html) >
/// [`common`](https://mavlink.io/en/messages/common.html) >
/// [`standard`]((https://mavlink.io/en/messages/standard.html))
/// [`minimal`]((https://mavlink.io/en/messages/minimal.html))
///
/// That means, that if you enabled `dlct-ardupilotmega` dialect but not `all`, then the former is the
/// most general canonical dialect, and it will be chosen as a default one.
///
/// Requires at least `dlct-minimal` dialect feature flag to be enabled.
///
/// Re-exported from [`mavspec::rust::DefaultDialect`].
///
/// ---
#[doc(inline)]
pub use mavspec::rust::DefaultDialect;

/// <sup>[`mavspec`](https://crates.io/crates/mavspec)</sup>
/// Tools for MAVLink [microservices](https://mavlink.io/en/services/)
///
/// Enabled by `msrv-utils-*` feature flags.
///
/// Re-exported from [`mavspec::rust::microservices`].
///
/// <section class="warning">
/// These feature is considered unstable. Use `unstable` feature flag to access this functionality.
/// </section>
///
/// ---
#[cfg(all(feature = "msrv-utils", feature = "unstable"))]
#[doc(inline)]
pub use mavspec::rust::microservices;

/// <sup>[`mavspec`](https://crates.io/crates/mavspec)</sup>
/// MAVLink message definitions
///
/// Requires `definitions` feature flag to be enabled.
///
/// <section class="warning">
/// Requires `std` feature flag to be enabled. Otherwise, the library won't compile.
/// </section>
///
/// Re-exported from [`mavspec::definitions`].
///
/// ---
#[cfg(feature = "definitions")]
#[doc(inline)]
pub use mavio::definitions;

/// <sup>[`mavspec`](https://crates.io/crates/mavspec)</sup>
/// MAVSpec procedural macros
///
/// Since derive macros relies on entities from [`mavspec::rust::spec`], you have to import
/// [`mavio::protocol::mavspec`](crate::protocol::mavspec) or use [`prelude`](crate::prelude). For example:
///
/// ```rust
/// #[cfg(feature = "derive")]
/// # {
/// use maviola::prelude::*; // This is necessary!!!
/// use maviola::protocol::derive::Enum;
///
/// #[derive(Enum)]
/// #[repr(u8)]
/// #[derive(Copy, Clone, Debug, Default)]
/// enum CustomEnum {
///     #[default]
///     DEFAULT = 0,
///     OptionA = 1,
///     OptionB = 2,
/// }
/// # }
/// ```
///
/// Requires `derive` feature flag to be enabled.
///
/// Re-exported from [`mavspec::rust::derive`].
///
/// ---
#[cfg(feature = "derive")]
#[doc(inline)]
pub use mavspec::rust::derive;
