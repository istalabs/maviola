Maviola
=======

A high-level [MAVLink](https://mavlink.io/en/) communication library written in Rust.

[🇺🇦](https://mavka.gitlab.io/home/a_note_on_the_war_in_ukraine/)
[![`repository`](https://img.shields.io/gitlab/pipeline-status/mavka/libs/maviola.svg?branch=main&label=repository)](https://gitlab.com/mavka/libs/maviola)
[![`crates.io`](https://img.shields.io/crates/v/maviola.svg)](https://crates.io/crates/maviola)
[![`docs.rs`](https://img.shields.io/docsrs/maviola.svg?label=docs.rs)](https://docs.rs/maviola/latest/maviola/)
[![`issues`](https://img.shields.io/gitlab/issues/open/mavka/libs/maviola.svg)](https://gitlab.com/mavka/libs/maviola/-/issues/)

<details>
<summary>
More on MAVLink
</summary>

MAVLink is a lightweight open protocol for communicating between drones, onboard components and ground control stations.
It is used by such autopilots like [PX4](https://px4.io) or [ArduPilot](https://ardupilot.org/#). MAVLink has simple and
compact serialization model. The basic abstraction is `message` which can be sent through a link (UDP, TCP, UNIX
socket, UART, whatever) and deserialized into a struct with fields of primitive types or arrays of primitive types.
Such fields can be additionally restricted by `enum` variants, annotated with metadata like units of measurements,
default or invalid values.

There are several MAVLink dialects. Official dialect definitions are
[XML files](https://mavlink.io/en/guide/xml_schema.html) that can be found in the MAVlink
[repository](https://github.com/mavlink/mavlink/tree/master/message_definitions/v1.0). Based on `message` abstractions,
MAVLink defines so-called [`microservices`](https://mavlink.io/en/services/) that specify how clients should respond on
a particular message under certain conditions or how they should initiate a particular action.
</details>

Maviola provides abstractions like communication nodes and takes care of **stateful** features of the MAVLink protocol,
such as sequencing, message time-stamping, automatic heartbeats, message signing, and so on. The key features are:

* Synchronous and asynchronous API. The latter is based on [Tokio](https://tokio.rs/).
* Both `MAVLink 1` and `MAVLink 2` protocol versions are supported, it is also possible to have protocol-agnostic
  channels that support both versions.
* Maviola supports all standard MAVLink dialects, controlled by corresponding cargo features.
* Additional custom dialects can be generated by [MAVSpec](https://gitlab.com/mavka/libs/mavspec).

This library is based on [Mavio](https://gitlab.com/mavka/libs/mavio), a low-level library with `no-std` support. If you
are looking for a solution for embedded devices, then Mavio probably would be a better option.

Usage
-----

> 📖 If you want to learn how to use Maviola, start from reading
> [Maviola Playbook](https://docs.rs/maviola/latest/maviola/docs). The following section is just a brief introduction.

This library provides both synchronous and asynchronous API. The synchronous API can be enabled by `sync` feature flag.
The asynchronous API is based on [Tokio](https://tokio.rs/), and can be enabled by `async` feature flag. The differences
between synchronous and asynchronous APIs are minimal, so you can easily switch between them, if necessary. It is also
possible to use both synchronous and asynchronous APIs in different parts of your project.

### Synchronous API

Install:

```shell
cargo add maviola --features sync
```

Create a synchronous TCP server that represents a particular MAVLink device:

```rust
use maviola::prelude::*;
use maviola::sync::prelude::*;

pub fn main() -> Result<()> {
    // Create a synchronous MAVLink node 
    // with MAVLink 2 protocol version
    let server = Node::sync::<V2>()
        .id(MavLinkId::new(17, 42))                     // Set device system and component IDs
        .connection(TcpServer::new("127.0.0.1:5600")?)  // Define connection settings
        .build()?;

    // Handle node events
    for event in server.events() {
        match event {
            // Handle a new peer
            Event::NewPeer(peer) => println!("new peer: {peer:?}"),
            // Handle a peer that becomes inactive
            Event::PeerLost(peer) => {
                println!("peer offline: {peer:?}");
                // Exit when all peers are disconnected
                if !server.has_peers() {
                    break;
                }
            }
            // Handle incoming MAVLink frame
            Event::Frame(frame, callback) => if server.validate_frame(&frame).is_ok() {
                // Handle heartbeat message
                if let Ok(Minimal::Heartbeat(msg)) = frame.decode::<Minimal>() {
                    // Respond with the same heartbeat message to all clients,
                    // except the one that sent this message
                    callback.respond_others(&server.next_frame(&msg)?)?;
                }
            }
            Event::Invalid(frame, err, callback) => {
                /* Handle invalid frame */
            }
        }
    }
}
```

### Asynchronous API

Install:

```shell
cargo add maviola --features async
```

Create an asynchronous TCP server that represents a particular MAVLink device:

```rust
use maviola::prelude::*;
use maviola::asnc::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Create an asynchronous MAVLink node
    // with MAVLink 2 protocol version
    let server = Node::asnc::<V2>()
        .id(MavLinkId::new(17, 42))             // Set device system and component IDs
        .connection(
            TcpServer::new("127.0.0.1:5600")?   // Define connection settings
        )
        .build().await?;

    // Subscribe to a stream of node events
    let mut events = server.events().unwrap();
    // Handle node events
    while let Some(event) = events.next().await {
        match event {
            // Handle a new peer
            Event::NewPeer(peer) => println!("new peer: {peer:?}"),
            // Handle a peer that becomes inactive
            Event::PeerLost(peer) => {
                println!("peer offline: {peer:?}");
                // Exit when all peers are disconnected
                if !server.has_peers().await {
                    break;
                }
            }
            // Handle incoming MAVLink frame
            Event::Frame(frame, callback) => if server.validate_frame(&frame).is_ok() {
                // Handle heartbeat message
                if let Ok(Minimal::Heartbeat(msg)) = frame.decode::<Minimal>() {
                    // Respond with the same heartbeat message to all clients,
                    // except the one that sent this message
                    callback.respond_others(&server.next_frame(&msg)?)?;
                }
            }
            Event::Invalid(frame, err, callback) => {
                /* Handle invalid frame */
            }
        }
    }
    Ok(())
}
```

Examples
--------

Basic examples can be found [here](maviola/examples).

License
-------

> Here we simply comply with the suggested dual licensing according to
> [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/about.html) (C-PERMISSIVE).

Licensed under either of

* Apache License, Version 2.0
  ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license
  ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

Contribution
------------

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
