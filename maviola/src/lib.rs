/*! # Maviola

A high-level [MAVLink](https://mavlink.io/en/) communication library written in Rust.

<span style="font-size:24px">[🇺🇦](https://mavka.gitlab.io/home/a_note_on_the_war_in_ukraine/)</span>
[![`repository`](https://img.shields.io/gitlab/pipeline-status/mavka/libs/maviola.svg?logo=gitlab&branch=main&label=repository)](https://gitlab.com/mavka/libs/maviola)
[![`crates.io`](https://img.shields.io/crates/v/maviola.svg)](https://crates.io/crates/maviola)
[![`docs.rs`](https://img.shields.io/docsrs/maviola.svg?label=docs.rs)](https://docs.rs/maviola/latest/maviola/)
[![`issues`](https://img.shields.io/gitlab/issues/open/mavka/libs/maviola.svg)](https://gitlab.com/mavka/libs/maviola/-/issues/)

Maviola provides abstractions such as communication nodes, networks, or devices and implements
_stateful_ features of MAVLink protocol: sequencing, message signing, automatic heartbeats, and so
on.

This library is a part of [Mavka](https://mavka.gitlab.io/home/) toolchain. It is based on
[Mavio](https://crates.io/crates/mavio), a low-level MAVLink library, and compatible with
[MAVSpec](https://crates.io/crates/mavspec) MAVLink dialects generator.

## 📖 Documentation

If you want to learn how to use Maviola, start from reading [Maviola Playbook](crate::docs).
This page contains only a brief introduction.

# Usage

Maviola provides both synchronous and asynchronous API. The synchronous API is available
in [`sync`] module and can be enabled by `sync` feature flag. The asynchronous API is based on
[Tokio](https://tokio.rs/), it can be found in [`asnc`] module and is enabled by `async` feature
flag.

Here is a simple example of a synchronous TCP server:

```rust,no_run
# #[cfg(not(feature = "sync"))] fn main() {}
# #[cfg(feature = "sync")]
# fn main() -> maviola::error::Result<()> {
use maviola::prelude::*;
use maviola::sync::prelude::*;

// Create a synchronous MAVLink node
// with MAVLink protocol version set to `V2`
let server = Node::sync::<V2>()
     // Set device system and component IDs
     .id(MavLinkId::new(17, 42))
     // Define connection settings
     .connection(TcpServer::new("127.0.0.1:5600")?)
     .build()?;

// Handle node events
for event in server.events() {
     match event {
         // Handle incoming MAVLink frame
         Event::Frame(frame, callback) => {
             // Handle heartbeat message
             if let Ok(DefaultDialect::Heartbeat(msg)) = frame.decode::<DefaultDialect>() {
                 // Respond with the same heartbeat message to all clients,
                 // except the one that sent this message
                 callback.broadcast(&server.next_frame(&msg)?)?;
             }
         }
         _ => {
             /* Handle other node events */
         }
     }
}
# Ok(()) }
```

You can learn more about this example in [Quickstart](crate::docs::a1__quickstart) section that
gets into details.

Also check [Overview](crate::docs::a2__overview), [Synchronous API](crate::docs::a3__sync_api),
and [Asynchronous API](crate::docs::a4__async_api) documentation sections for details on how to
use different types of API.

# Features

Maviola is designed to hide most of its functionality under corresponding feature flags. If you
need certain features, you have to explicitly opt-in.

## API Modes

* `sync` enables synchronous API (see [`sync`] module and
   [Synchronous API](crate::docs::a3__sync_api)).
* `async` enables asynchronous API (see [`asnc`] module and
   [Asynchronous API](crate::docs::a4__async_api)).

These features are not mutually exclusive, you can use both synchronous and asynchronous API in
different parts of the project.

## Unstable Features

Some parts of the API are still considered to be unstable and available only under the
`unstable` feature flag. We mark unstable and experimental entities with <sup>`⍚`</sup> in
documentation.

## Embedded Devices

Maviola is based on [Mavio](https://crates.io/crates/mavio), a low-level library with
`no-std` support. If you are looking for a solution for embedded devices, then Mavio would
probably be a better option.

# MAVLink Protocol

Protocol entities reside in the [`protocol`] module.

## Dialects

Maviola packages standard MAVLink dialects under corresponding feature flags. It is possible
to define your own dialects with XML message definitions using
[MAVSpec](https://crates.io/crates/mavspec) or even create your ad-hoc dialects using pure
Rust.

Check [Dialects](crate::docs::a2__overview#dialects) documentation section for details.

## Microservices

We utilise [MAVSpec](https://crates.io/crates/mavspec) ability to generate MAVLink
[microservices](https://mavlink.io/en/services/) as sub-dialects. Use `msrv-*` feature flags to
enable specific microservices.

We also re-export additional microservice utils as [`protocol::microservices`]. You should enable
the corresponding `msrv-utils-*` feature flag to access such functionality.

## Message Definitions

You may access metadata for MAVLink message definitions by enabling `definitions` feature flag.
The metadata is available at [`protocol::definitions`].

# Feature Flags
*/
#![doc = document_features::document_features!()]
//
#![warn(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![doc(
    html_logo_url = "https://gitlab.com/mavka/libs/maviola/-/raw/main/avatar.png?ref_type=heads",
    html_favicon_url = "https://gitlab.com/mavka/libs/maviola/-/raw/main/avatar.png?ref_type=heads"
)]
#![cfg_attr(not(all(feature = "sync", feature = "async")), allow(unused_imports))]
#![cfg_attr(not(all(feature = "sync", feature = "async")), allow(dead_code))]

#[cfg(feature = "async")]
pub mod asnc;
pub mod core;
pub mod error;
pub mod prelude;
pub mod protocol;
#[cfg(feature = "sync")]
pub mod sync;

// #[doc(inline)]
// pub use protocol::{default_dialect, derive, dialects, DefaultDialect};

#[cfg(any(doc, doctest, rustdoc))]
#[cfg(all(feature = "sync", feature = "async"))]
pub mod docs;

#[cfg(feature = "test_utils")]
#[doc(hidden)]
pub mod test_utils;
