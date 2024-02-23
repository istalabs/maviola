//! # Maviola
//!
//! A high-level [MAVLink](https://mavlink.io/en/) communication library written in Rust. Maviola
//! provides abstractions like communication nodes and namespaces and takes care of **stateful**
//! features of the MAVLink protocol, such as sequencing, message time-stamping, automatic
//! heartbeats, simplifies message signing, and so on.
//!
//! Maviola is based on [Mavio](https://gitlab.com/mavka/libs/mavio), a low-level library with
//! `no-std` support. If you are looking for a solution for embedded devices, then Mavio would be a
//! better option.
//!
//! > **⚠ WIP**
//! >
//! > Maviola is still under heavy development. The aim is to provide API similar to
//! > [`gomavlib`](https://github.com/bluenviron/gomavlib) with additional support for essential MAVLink
//! > ["microservices"](https://mavlink.io/en/services/) such as [heartbeat](https://mavlink.io/en/services/heartbeat.html),
//! > [parameter protocol](https://mavlink.io/en/services/parameter.html) and
//! > [commands](https://mavlink.io/en/services/command.html).
//! >
//! > This is project stub. We intentionally do not publish early versions of API to avoid confusion and massive
//! > breaking changes.

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
