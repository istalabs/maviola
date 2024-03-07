//! # Maviola
//!
//! A high-level [MAVLink](https://mavlink.io/en/) communication library written in Rust.
//!
//! <span style="font-size:24px">[🇺🇦](https://mavka.gitlab.io/home/a_note_on_the_war_in_ukraine/)</span>
//! [![`repository`](https://img.shields.io/gitlab/pipeline-status/mavka/libs/maviola.svg?branch=main&label=repository)](https://gitlab.com/mavka/libs/maviola)
//! [![`crates.io`](https://img.shields.io/crates/v/maviola.svg)](https://crates.io/crates/maviola)
//! [![`docs.rs`](https://img.shields.io/docsrs/maviola.svg?label=docs.rs)](https://docs.rs/maviola/latest/maviola/)
//! [![`issues`](https://img.shields.io/gitlab/issues/open/mavka/libs/maviola.svg)](https://gitlab.com/mavka/libs/maviola/-/issues/)
//!
//! Maviola provides abstractions such as communication node and implements **stateful** features
//! of MAVLink protocol: sequencing, message signing, automatic heartbeats, and so on.
//!
//! This library is a part of [Mavka](https://mavka.gitlab.io/home/) toolchain. It is based on
//! [Mavio](https://gitlab.com/mavka/libs/mavio), a low-level MAVLink library, and compatible with
//! [MAVSpec](https://gitlab.com/mavka/libs/mavspec) MAVLink dialects generator.
//!
//! ## Usage
//!
//! This library provides both synchronous and asynchronous API. The synchronous API is available
//! in [`sync`] module and can be enabled by `sync` feature flag. The asynchronous API is based on
//! [Tokio](https://tokio.rs/), it can be found in [`asnc`] module and is enabled by `async` feature
//! flag.
//!
//! The central Maviola abstraction is [`Node`](core::node::Node). A node represents a connection to
//! MAVLink network. Nodes which represent MAVLink components with defined system and component IDs
//! are called **edge** nodes. Unidentified nodes are called **proxy** nodes. The former can send
//! automatic heartbeats and perform other actions that does not require initiative from the user.
//!
//! ⓘ We suggest to use [`prelude`] with the corresponding [`sync::prelude`] / [`asnc::prelude`]
//! whenever possible. This will import the most useful abstractions.
//!
//! ### Synchronous API
//!
//! Install:
//!
//! ```shell
//! cargo add maviola --features sync
//! ```
//!
//! <details>
//! <summary>TCP server example</summary>
//!
//! Create a synchronous TCP server that represents a particular MAVLink device
//! ([`EdgeNode`](sync::node::EdgeNode)):
//!
//! ```rust,no_run
//! use maviola::prelude::*;
//! use maviola::sync::prelude::*;
//!
//! # fn main() -> Result<()> {
//! // Create a MAVLink node
//! let server = Node::builder()
//!     .version(V2)                                    // Set protocol version to `V2`
//!     .dialect::<Minimal>()                           // Set MAVLink dialect to `minimal`
//!     .id(MavLinkId::new(17, 42))                     // Set device system and component IDs
//!     .connection(TcpServer::new("127.0.0.1:5600")?)  // Define connection settings
//!     .build()?;
//!
//! // Handle node events
//! for event in server.events() {
//!     match event {
//!         // Handle a new peer
//!         Event::NewPeer(peer) => println!("new peer: {peer:?}"),
//!         // Handle a peer that becomes inactive
//!         Event::PeerLost(peer) => {
//!             println!("peer offline: {peer:?}");
//!             // Exit when all peers are disconnected
//!             if !server.has_peers() {
//!                 break;
//!             }
//!         }
//!         // Handle incoming MAVLink frame
//!         Event::Frame(frame, callback) => if server.validate_frame(&frame).is_ok() {
//!             // Handle heartbeat message
//!             if let Ok(Minimal::Heartbeat(msg)) = frame.decode::<Minimal>() {
//!                 // Respond with the same heartbeat message to all clients,
//!                 // except the one that sent this message
//!                 callback.respond_others(&server.next_frame(&msg)?)?;
//!             }
//!         }
//!         Event::Invalid(frame, err, callback) => {
//!             /* Handle invalid frame */
//!         }
//!     }
//! }
//! # Ok(()) }
//! ```
//! </details>
//!
//! ### Asynchronous API
//!
//! Install:
//!
//! ```shell
//! cargo add maviola --features async
//! ```
//!
//! <details>
//! <summary>TCP server example</summary>
//!
//! Create an asynchronous TCP server that represents a particular MAVLink device
//! ([`EdgeNode`](asnc::node::EdgeNode)):
//!
//! ```rust,no_run
//! use maviola::prelude::*;
//! use maviola::asnc::prelude::*;
//!
//! # #[tokio::main] async fn main() -> Result<()> {
//! // Create a MAVLink node
//! let server = Node::builder()
//!     .version(V2)                            // Set protocol version to `V2`
//!     .dialect::<Minimal>()                   // Set MAVLink dialect to `minimal`
//!     .id(MavLinkId::new(17, 42))             // Set device system and component IDs
//!     .async_connection(
//!         TcpServer::new("127.0.0.1:5600")?   // Define connection settings
//!     )
//!     .build().await?;
//!
//! // Subscribe to a stream of node events
//! let mut events = server.events().unwrap();
//! // Handle node events
//! while let Some(event) = events.next().await {
//!     match event {
//!         // Handle a new peer
//!         Event::NewPeer(peer) => println!("new peer: {peer:?}"),
//!         // Handle a peer that becomes inactive
//!         Event::PeerLost(peer) => {
//!             println!("peer offline: {peer:?}");
//!             // Exit when all peers are disconnected
//!             if !server.has_peers().await {
//!                 break;
//!             }
//!         }
//!         // Handle incoming MAVLink frame
//!         Event::Frame(frame, callback) => if server.validate_frame(&frame).is_ok() {
//!             // Handle heartbeat message
//!             if let Ok(Minimal::Heartbeat(msg)) = frame.decode::<Minimal>() {
//!                 // Respond with the same heartbeat message to all clients,
//!                 // except the one that sent this message
//!                 callback.respond_others(&server.next_frame(&msg)?)?;
//!             }
//!         }
//!         Event::Invalid(frame, err, callback) => {
//!             /* Handle invalid frame */
//!         }
//!     }
//! }
//! # Ok(()) }
//! ```
//! </details>
//!
//! ## Features
//!
//! Maviola is designed to hide most of its functionality under corresponding feature flags. If you
//! need certain features, you have to explicitly opt-in.
//!
//! ### API Modes
//!
//! * `sync` enables synchronous API (see [`sync`]).
//! * `async` enables asynchronous API (see [`asnc`]).
//!
//! These features are not mutually exclusive, you can use both synchronous and asynchronous API in
//! different parts of the project.
//!
//! ### MAVLink Dialects
//!
//! Standard MAVLink dialect can be enabled by the corresponding feature flags. The `minimal` is
//! always enabled.
//!
//! <details>
//! <summary>Available MAVLink dialects</summary>
//!
//! These MAVLink dialects are re-exported by [Mavio](https://gitlab.com/mavka/libs/mavio) and
//! available in [`dialects`] module:
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
//! </details>
//!
//! Custom MAVLink dialects can be generated from XML message definitions using
//! [MAVSpec](https://gitlab.com/mavka/libs/mavspec). Check MAVSpec documentation for details.
//!
//! ### Unstable Features
//!
//! Some parts of the API are still considered to be unstable and available only under the
//! `unstable` feature flag. We mark unstable and experimental entities with <sup>`⍚`</sup> in
//! documentation.
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
// #[allow(async_fn_in_trait)]
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
