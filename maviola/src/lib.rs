//! # Maviola
//!
//! A high-level [MAVLink](https://mavlink.io/en/) communication library written in Rust. Maviola
//! provides abstractions like communication nodes and namespaces and takes care of **stateful**
//! features of the MAVLink protocol, such as sequencing, message time-stamping, automatic
//! heartbeats, simplifies message signing, and so on.
//!
//! This library is a part of [Mavka](https://mavka.gitlab.io/home/) toolchain. It is based on
//! [Mavio](https://gitlab.com/mavka/libs/mavio), a low-level MAVLink library, and compatible with
//! [MAVSpec](https://gitlab.com/mavka/libs/mavspec) MAVLink dialects generator.
//!
//! ## Features
//!
//! Maviola is designed to hide most of its functionality under corresponding feature flags. If you
//! need certain features, you have to explicitly opt-in.
//!
//! ### API Modes
//!
//! Maviola supports both synchronous and asynchronous API:
//!
//! * `sync` enables synchronous API (see [`sync`]).
//! * `async` enables asynchronous API (see [`asnc`]).
//!
//! ### MAVLink Dialects
//!
//! Standard MAVLink dialect can be enabled by the corresponding feature flags. The `minimal` is
//! always enabled.
//!
//! * [`minimal`]((https://mavlink.io/en/messages/minimal.html)) — minimal dialect required to
//!   expose your presence to other MAVLink devices (this dialect is enabled by default).
//! * [`standard`](https://mavlink.io/en/messages/standard.html) — a superset of `minimal` dialect,
//!   that expected to be used by almost all flight stack.
//! * [`common`](https://mavlink.io/en/messages/common.html) — minimum viable dialect with most of
//!   the features, a building block for other future-rich dialects.
//! * [`ardupilotmega`](https://mavlink.io/en/messages/common.html) — feature-full dialect used by
//!   [ArduPilot](http://ardupilot.org). In most cases this dialect is the go-to choice if you want
//!   to recognize almost all MAVLink messages used by existing flight stacks.
//! * [`all`](https://mavlink.io/en/messages/all.html) — meta-dialect which includes all other
//!   standard dialects including these which were created for testing purposes. It is guaranteed
//!   that namespaces of the dialects in `all` family do not collide.
//! * Other dialects from MAVLink XML [definitions](https://github.com/mavlink/mavlink/tree/master/message_definitions/v1.0):
//!   `asluav`, `avssuas`, `csairlink`, `cubepilot`, `development`, `icarous`, `matrixpilot`,
//!   `paparazzi`, `ualberta`, `uavionix`. These do not include `python_array_test` and `test`
//!   dialects which should be either generated manually or as a part of `all` meta-dialect.
//!
//! ### Unstable Features
//!
//! Some parts of the API are still considered to be unstable and available only under the
//! `unstable` feature flag.
//!
//! ## Embedded Devices
//!
//! Maviola is based on [Mavio](https://gitlab.com/mavka/libs/mavio), a low-level library with
//! `no-std` support. If you are looking for a solution for embedded devices, then Mavio would
//! probably be a better option.

#![warn(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![doc(
    html_logo_url = "https://gitlab.com/mavka/libs/maviola/-/raw/main/avatar.png?ref_type=heads",
    html_favicon_url = "https://gitlab.com/mavka/libs/maviola/-/raw/main/avatar.png?ref_type=heads"
)]

#[cfg(feature = "async")]
#[allow(async_fn_in_trait)]
#[allow(unused_imports)]
#[allow(dead_code)]
pub mod asnc;
pub mod core;
pub mod prelude;
pub mod protocol;
#[cfg(feature = "sync")]
pub mod sync;

pub(crate) extern crate mavio;

#[doc(inline = true)]
/// <sup>[`mavio`](https://crates.io/crates/mavio)</sup>
/// MAVLink dialects
pub use mavio::dialects;
