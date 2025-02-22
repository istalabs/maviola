/*!
# 📖 1.2. Overview

<em>[← Quickstart](crate::docs::a1__quickstart) | [Synchronous API →](crate::docs::a3__sync_api)</em>

This chapter contains a general overview of the Maviola API. We suggest to read it briefly even if
you are going to use only specific part of the API.

## Contents

1. [Why Maviola?](#why-maviola)
1. [Choosing Your API](#choosing-your-api)
1. [Prelude](#prelude)
1. [MAVLink Protocol](#mavlink-protocol)
1. [Node](#node)
    1. [Sending Messages](#sending-messages)
    1. [Receiving Frames](#receiving-frames)
    1. [Responding to Frames](#responding-to-frames)
    1. [Protocol-Agnostic Nodes](#protocol-agnostic-nodes)
1. [MAVLink 2 Features](#mavlink-2-features)
    1. [Message Signing](#message-signing)
    1. [Compatibility Checks](#compatibility-checks)
1. [Networks & Routing](#networks--routing)
    1. [Retry Logic](#retry-logic)
    1. [Networks as Collections of Nodes](#networks-as-collections-of-nodes)
    1. [Routing](#routing)
1. [Dialects](#dialects)
    1. [Canonical Dialects](#canonical-dialects)
    1. [Default Dialect](#default-dialect)
    1. [Custom Dialects](#custom-dialects)
    1. [Ad-hoc Dialects](#ad-hoc-dialects)

## Why Maviola?

When someone writes _another_ library for the similarly same purpose as the existing one, it is
important to explain the reasons behind the edifice. Yes, there is already a
[rust-mavlink](https://github.com/mavlink/rust-mavlink) library. However, there are certain reasons
we believe it shouldn`t be a default choice for building Rust ecosystem for MAVLink.

But before explaining ourselves, we want to express our gratitude to the people, who've built
`rust-mavlink`. We believe that certain decision they've made are right and elegant. Some of these
decision we've copied and some were rediscovered by us to our great surprise. Still.

Beyond the extremely slow compilation time of `rust-mavlink` that can be speed up to 5-6 times by
introducing relatively simple optimizations, the most important difference between `rust-mavlink`
and Maviola is that the former is a purely low-level library. While the latter is a high-level
library that is interoperable with low-level `no_std` and no-alloc
[Mavio](https://gitlab.com/mavka/libs/mavio) crate. Which means that you can write you code for
embedded devices using Mavio, and then use more convenient and powerful Maviola library on your
companion computer or inside your communication infrastructure.

Then why don't we simply extended `rust-mavlink`? This is a valid question. We've decided to build
our solution from the ground for two reasons. First, `rust-mavlink` does not expose neither XML
analyser for dialect definitions nor code generator. That means, if you built a crate based on the
`rust-mavlink`, it would be harder to add additional dialects and impossible to extend interfaces
for existing MAVLink messages. Our approach is different. Instead of a library, we've decided to
build a toolchain that includes [MAVInspect](https://gitlab.com/mavka/libs/mavinspect) for
XML parsing and [MAVSpec](https://gitlab.com/mavka/libs/mavspec) for code generation. The latter
allows to add additional feature-gated capabilities to generated MAVLink messages without changing
communication libraries. For example, one may extend MAVSpec by adding functionality for storing
MAVLink messages into an SQL database. Having a code generator as a first-class citizen allows
to build tools that interoperate seamlessly between embedded and conventional computers. This also
allows to implement interesting features like [Ad-hoc Dialects](crate::docs::c4__ad_hoc_dialects)
defined purely by Rust code.

Another somewhat opinionated reason for building our toolchain is that we believe that Rust
libraries should be compelling. And from our perspective the current `rust-mavlink` solution is not
compelling enough. What do we mean? The existing MAVLink ecosystem is relatively old and some
languages already have their bindings. First, there is an outstanding C++-based
[MAVSDK](https://github.com/mavlink/MAVSDK) that enables arbitrary programming language support
based on [Protobuf](https://protobuf.dev/). There is an elegant
[gomavlib](https://github.com/bluenviron/gomavlib) for Go from
[bluenviron](https://github.com/bluenviron). Even the [Great Snake](https://www.python.org) has it
all. That means that we don't just compete _within_ Rust ecosystem but rather _for_ Rust
ecosystem. And at the moment we have to admit that the latter is not compelling enough to convince
people proficient in other languages to adopt Rust for MAVLink-related tasks. From the very
beginning we were doing our best to carefully design APIs, provide extensive documentation and even
clarify certain parts of the MAVLink protocol that are hard to find in the
[official documentation](https://mavlink.io/en/).

We hope that our work will be helpful both for those who are already using Rust in their MAVLink
infrastructure and those who are considering to adopt Rust for their next endeavour.

## Choosing Your API

Maviola provides both synchronous and asynchronous API which are boringly similar to each other.
In most cases by changing a few constructors and spreading `async` and `await` you can turn
synchronous code into the asynchronous one. Still, we want to outline certain differences which
may be important to you.

First of all, asynchronous API is based on [Tokio](https://tokio.rs/). If for some reason you are
using another asynchronous runtime, then it probably won't work for you out of the box. For the same
reason you might refrain from using asynchronous API to reduce the amount of dependencies.

When comparing two current implementation, we must confess that asynchronous API is somewhat more
elaborated. The main reason is that despite being slightly faster (10-15%) current synchronous API
is way more eager. Currently, we use several threads per connection. That means that while you may
squeeze some performance, the cost may be too high. By the rules of thumb, we suggest to use
synchronous API when you either have relatively few connections (dozens) or you are allowed to use
all cores of your machine. Especially in the latter case.

Another consideration related to previous one is that there is simply no way to run synchronous API
using one thread. At the same time benchmarks for single-threaded Tokio runtime show considerable
performance.

In the next versions we may introduce a more granular approach to threads by the means of thread
pools. We are certainly looking into it. But since [Tokio](https://tokio.rs/) is already a thread
pool with a fine-grained control over the resources we are not rushing forward. Such changes require
extensive benchmarks and significant efforts.

## Prelude

We suggest to use [`prelude`] with the corresponding [`sync::prelude`] / [`asnc::prelude`]
whenever possible. The [`prelude`] will import common entities and traits, while the latter two
should be used in the context of a particular API mode.

The common [`prelude`] is somewhat bulky. However, we found it useful, especially in the areas,
where nodes are constructed or manipulated. It is possible and makes sense to use [`sync::prelude`]
/ [`asnc::prelude`] without the common [`prelude`].

ⓘ Synchronous and asynchronous preludes are not compatible.

## MAVLink Protocol

MAVLink protocol is relatively complex. We put all entities related to it into [`protocol`] module.
The main abstraction of the protocol is [`Frame`]. Everything you are going to do will be related to
sending or receiving MAVLink frames.

Frames can be encoded and decoded from messages. Each message in the known dialect implements
[`Message`] trait, that allows to create a [`Frame`].

MAVLink dialect is a set of messages. In Maviola this is an enum that implements [`Dialect`] trait.
In most cases you will be interested in decoding incoming frames into a dialect and then pattern
match for the message you are interested in. You can use [`Dialect::message_info`] to check whether
a particular [`Frame`] with [`message_id`] belongs to a dialect to skip decoding all frames.

You can attempt to decode any frame to any dialect using [`Frame::decode`].

A simple example of a dialect is [`Minimal`] dialect included into all meaningful canonical MAVLink
dialects.

## Node

The central Maviola abstraction is [`Node`]. A node represents a connection to
MAVLink network. Nodes which represent MAVLink components with defined system and component `ID`s
are called **edge** nodes. Unidentified nodes are called **proxy** nodes. The former can send
automatic heartbeats and perform other actions that does not require initiative from the user.

To construct an edge node we need to define its system `ID`, component `ID` and connection:

```rust,no_run
use maviola::prelude::*;
use maviola::sync::prelude::*;

let node = Node::sync::<V2>()
    .id(MavLinkId::new(1, 17))
    .connection(TcpServer::new("127.0.0.1:5600").unwrap())
    .build().unwrap();
```

Asynchronous nodes can be constructed in a similar way:

```rust,no_run
use maviola::prelude::*;
use maviola::asnc::prelude::*;
# #[tokio::main] async fn main() {

let node = Node::asnc::<V2>()
    .id(MavLinkId::new(1, 17))
    .connection(TcpServer::new("127.0.0.1:5600").unwrap())
    .build().await.unwrap();
# }
```

### Sending Messages

Once node is created, we can send messages:

```rust,no_run
# use maviola::prelude::*;
# use maviola::sync::prelude::*;
# let node = Node::sync::<V2>()
#    .id(MavLinkId::new(1, 17))
#    .connection(TcpServer::new("127.0.0.1:5600").unwrap())
#    .build().unwrap();
#
use maviola::protocol::dialects::minimal::messages::Heartbeat;

node.send(&Heartbeat::default()).unwrap();
```

We can also send frames directly:

```rust,no_run
# use maviola::prelude::*;
# use maviola::sync::prelude::*;
# let node = Node::sync::<V2>()
#    .id(MavLinkId::new(1, 17))
#    .connection(TcpServer::new("127.0.0.1:5600").unwrap())
#    .build().unwrap();
# use maviola::protocol::dialects::minimal::messages::Heartbeat;
#
let frame = node.next_frame(&Heartbeat::default()).unwrap();
node.send_frame(&frame).unwrap();
```

Here we've created a new frame from message using [`Node::next_frame`]. This was done for the sake
of simplicity. In most cases you are going to use [`Node::send_frame`] to send frames you've
received from somewhere else.

### Receiving Frames

The suggested way for receiving incoming frames is to use `events` method:

```rust,no_run
# use maviola::prelude::*;
# use maviola::sync::prelude::*;
# let node = Node::sync::<V2>()
#    .id(MavLinkId::new(1, 17))
#    .connection(TcpServer::new("127.0.0.1:5600").unwrap())
#    .build().unwrap();
#
for event in node.events() {
    match event {
        Event::Frame(frame, _) => {
            let message = frame.decode::<DefaultDialect>().unwrap();
            match message {
                DefaultDialect::Heartbeat(heartbeat) => {
                    println!("Received heartbeat: {:?}", heartbeat);
                }
                /* process other messages */
                # _ => {}
            }
        }
        /* process other events including invalid frames */
        # _ => {}
    }
}
```

For asynchronous API `events` returns a [stream](https://tokio.rs/tokio/tutorial/streams) instead of iterator:

```rust,no_run
# #[tokio::main] async fn main() {
# use maviola::prelude::*;
# use maviola::asnc::prelude::*;
# let node = Node::asnc::<V2>()
#    .id(MavLinkId::new(1, 17))
#    .connection(TcpServer::new("127.0.0.1:5600").unwrap())
#    .build().await.unwrap();
#
let mut events = node.events().unwrap();
while let Some(event) = events.next().await {
    match event {
        Event::Frame(frame, _) => {
            let message = frame.decode::<DefaultDialect>().unwrap();
            match message {
                DefaultDialect::Heartbeat(heartbeat) => {
                    println!("Received heartbeat: {:?}", heartbeat);
                }
                /* process other messages */
                # _ => {}
            }
        }
        /* process other events including invalid frames */
        # _ => {}
    }
}
# }
```

Synchronous and asynchronous APIs have their own event: [`sync::Event`] and [`asnc::Event`]. Check
corresponding documentation for all available events.

It is also possible to receive only valid frames:

```rust,no_run
# use maviola::prelude::*;
# use maviola::sync::prelude::*;
# let node = Node::sync::<V2>()
#    .id(MavLinkId::new(1, 17))
#    .connection(TcpServer::new("127.0.0.1:5600").unwrap())
#    .build().unwrap();
#
let (frame, _) = node.recv_frame().unwrap();
```

In case of asynchronous API we need mutable access to receive a frame:

```rust,no_run
# #[tokio::main] async fn main() {
# use maviola::prelude::*;
# use maviola::asnc::prelude::*;
# let node = Node::asnc::<V2>()
#    .id(MavLinkId::new(1, 17))
#    .connection(TcpServer::new("127.0.0.1:5600").unwrap())
#    .build().await.unwrap();
#
let mut node = node;
let (frame, _) = node.recv_frame().await.unwrap();
# }
```

### Responding to Frames

It is possible to respond directly to a received frame using `callback`:

```rust,no_run
# use maviola::prelude::*;
# use maviola::sync::prelude::*;
# let node = Node::sync::<V2>()
#    .id(MavLinkId::new(1, 17))
#    .connection(TcpServer::new("127.0.0.1:5600").unwrap())
#    .build().unwrap();
#
let (frame, callback) = node.recv_frame().unwrap();
callback.broadcast(&frame).unwrap();
```

The `broadcast` method will send obtained frame to all other channels except the one that sent the
original frame. For example, in the case of a TCP server, all other clients except the original one
will receive this frame.

Asynchronous API looks boringly similar:

```rust,no_run
# #[tokio::main] async fn main() {
# use maviola::prelude::*;
# use maviola::asnc::prelude::*;
# let mut node = Node::asnc::<V2>()
#    .id(MavLinkId::new(1, 17))
#    .connection(TcpServer::new("127.0.0.1:5600").unwrap())
#    .build().await.unwrap();
#
let (frame, callback) = node.recv_frame().await.unwrap();
callback.broadcast(&frame).unwrap();
# }
```

Each API kind has their own callback implementation. However, both asynchronous and synchronous
callbacks implement the same [`CallbackApi`].

### Protocol-Agnostic Nodes

In each case above we've defined MAVLink protocol version as [`V2`]. It is possible to create a
protocol-agnostic node (synchronous API):

```rust,no_run
use maviola::prelude::*;
use maviola::sync::prelude::*;

let node = Node::builder()
    .sync()
    .id(MavLinkId::new(1, 17))
    .connection(TcpServer::new("127.0.0.1:5600").unwrap())
    .build().unwrap();
```

Then we can receive [`Versionless`] frames from this node:

```rust,no_run
# use maviola::prelude::*;
# use maviola::sync::prelude::*;
# let node = Node::builder()
#    .sync()
#    .id(MavLinkId::new(1, 17))
#    .connection(TcpServer::new("127.0.0.1:5600").unwrap())
#    .build().unwrap();
#
let (frame, _) = node.recv_frame().unwrap();
let frame_v2: Frame<V2> = frame.try_into_versioned().unwrap();
```

We also can send frames of a specific protocol version:

```rust,no_run
# use maviola::prelude::*;
# use maviola::sync::prelude::*;
# let node = Node::builder()
#    .sync()
#    .id(MavLinkId::new(1, 17))
#    .connection(TcpServer::new("127.0.0.1:5600").unwrap())
#    .build().unwrap();
#
use maviola::protocol::dialects::minimal::messages::Heartbeat;

let frame = node.next_frame_versioned::<V2>(&Heartbeat::default()).unwrap();
node.send_frame(&frame).unwrap();
```

The drawback is that version-agnostic nodes can't be activated for sending heartbeats since it is
not clear which version of the heartbeat we need to send. However, we believe, that this is a minor
inconvenience since `MAVLink 1` devices are extremely rare and in most of the cases you would want
to set up a bridge between `MAVLink 1` and `MAVLink 2` networks. For example, by upgrading frames
using [`Frame::upgrade_with_crc_extra`].

If you really need to support for both `MAVLink 1` and `MAVLink 2` protocols simultaneously, then
you are welcome to create an [issue](https://gitlab.com/mavka/libs/maviola/-/issues/) or submit a
[pull-request](https://gitlab.com/mavka/libs/maviola/-/merge_requests).

## MAVLink 2 Features

In most of the examples we've silently assumed, that we are communicating using
[MAVLink 2](https://mavlink.io/en/guide/mavlink_2.html) protocol version. This is not accidental.
Only few soon-to-be-outdated devices are still using `MAVLink 1`. The second generation of a
protocol has a larger namespace for possible messages, supports additional features, and in some
scenarios can be even faster.

Maviola supports and embraces moder features of the second MAVLink protocol generation. It allows
to sign messages and supports automatic setting and verification of compatibility and
incompatibility flags.

### Message Signing

To sign and validate a messages, just add [`FrameSigner`] to your node configuration. For example,
the following node will strictly check all incoming messages and sign all unsigned outgoing frames:

```rust,no_run
# use maviola::prelude::*;
# use maviola::sync::prelude::*;
#
let node = Node::sync::<V2>()
    .id(MavLinkId::new(1, 17))
    .connection(TcpServer::new("127.0.0.1:5600").unwrap())
    .signer(FrameSigner::builder()
        .link_id(11)
        .key("secure key")
        .incoming(SignStrategy::Strict)
        .outgoing(SignStrategy::Sign)
    )
    .build().unwrap();
```

To learn more about message signing check the [Message Signing](crate::docs::b2__signing) section
of this documentation.

### Compatibility Checks

MAVLink protocol defines two sets of flags for compatibility and incompatibility. Compatibility
flags are the hints for the receiver which may alter the way how frames are handled. Incompatibility
flags in their turn define the set of features, that receiver must support in order to consume a
message.

To control behavior related to compatibility / incompatibility flags, add [`CompatProcessor`] to a
node. For example:

```rust,no_run
# use maviola::prelude::*;
# use maviola::sync::prelude::*;
#
use maviola::protocol::{IncompatFlags, CompatFlags};

let node = Node::sync::<V2>()
    .connection(TcpServer::new("127.0.0.1:5600").unwrap())
    .compat(CompatProcessor::builder()
        .incompat_flags(IncompatFlags::BIT_2 | IncompatFlags::BIT_5)
        .compat_flags(CompatFlags::BIT_3 | CompatFlags::BIT_4)
        .incoming(CompatStrategy::Reject)
        .outgoing(CompatStrategy::Enforce)
    )
    .build().unwrap();
```

This will reject all incompatible incoming frames based on the incompatibility flags and set both
incompatibility and compatibility flags for outgoing frames to specified values.

To learn more about compatibility management read the
[Compatibility Checks](crate::docs::b3__compat_checks) section of this documentation.

## Networks & Routing

A key Maviola feature is ability to support multiple connections and route messages between them.
Consider an example:

```rust,no_run
# use maviola::prelude::*;
# use maviola::sync::prelude::*;
#
let node = Node::sync::<V2>()
    .id(MavLinkId::new(1, 17))
    .connection(
        Network::sync()
            .add_connection(TcpServer::new("127.0.0.1:5600").unwrap())
            .add_connection(UdpClient::new("10.98.0.1:14550").unwrap())
    )
    .build().unwrap();
```

Here we created a node. But instead of one specific connection we provided a [`Network`] that
contains several connections.

### Retry Logic

One of the advantages of networks is that they can monitor their connections and restore them, when
possible. Consider a following example:

```rust,no_run
# use maviola::prelude::*;
# use maviola::sync::prelude::*;
#
use std::time::Duration;

let node = Node::sync::<V2>()
    .id(MavLinkId::new(1, 17))
    .connection(
        Network::sync()
            .add_connection(TcpClient::new("127.0.0.1:5600").unwrap())
            .retry(RetryStrategy::Always(Duration::from_millis(500)))
    )
    .build().unwrap();
```

When the server at `127.0.0.1:5600` goes down for some reason, then our client connection will
attempt to restart itself with interval of 500 milliseconds.

### Networks as Collections of Nodes

Under the hood network is a collection of proxy nodes. We can write
this explicitly adding additional interesting behavior to these nodes:

```rust,no_run
# use maviola::prelude::*;
# use maviola::sync::prelude::*;
#
let node = Node::sync::<V2>()
    .connection(
        Network::sync()
            .add_node(
                Node::sync()
                    .connection(TcpServer::new("127.0.0.1:5600").unwrap())
                    .signer(FrameSigner::builder()
                        .link_id(11)
                        .key("secure key")
                        .outgoing(SignStrategy::Strip)
                        .incoming(SignStrategy::Sign)
                    )
            )
            .add_node(
                Node::sync()
                    .connection(UdpClient::new("10.98.0.1:14550").unwrap())
                    .signer(FrameSigner::builder()
                        .link_id(11)
                        .key("secure key")
                        .outgoing(SignStrategy::Strict)
                        .incoming(SignStrategy::Strict)
                    )
            )
    )
    .build().unwrap();
```

The above example creates a network with two connections:

* The first connection is a TCP server that signs incoming messages and strips signatures from
  outgoing frames. Let's call this connection "trusted".
* The second connection is a UDP server that communicates only signed messages. We are going to
  call this connection "unsecure".

This is a common scenario for proxies running on a companion computer. We take trusted messages from
the flight controller, sign them, and broadcast to the entire network. Since flight controller
usually don't have a capability to verify frames, we strip signatures in the trusted network.
To avoid passing unauthorized messages from inherently dangerous outer world that full of germs,
predating flowers, and morning mists we reject all incorrectly signed frames.

It is also possible to create networks of networks and so on. But let's refrain from falling into
this rabbit hole for now.

### Routing

Once we've creating a network, it is tempting to define a set of rules for routing frames between
connections.

Say, we have the following network setup:

```rust,no_run
# use maviola::prelude::*;
# use maviola::sync::prelude::*;
#
let conn_from = TcpServer::new("127.0.0.1:5601").unwrap();
let conn_to = TcpServer::new("127.0.0.1:5602").unwrap();
```

What we want, is to route messages from the first connection to the second one. The following code
shows how to do that:

```rust,no_run
# use maviola::prelude::*;
# use maviola::sync::prelude::*;
#
# let conn_from = TcpServer::new("127.0.0.1:5601").unwrap();
# let conn_to = TcpServer::new("127.0.0.1:5602").unwrap();
#
// Obtain connection IDs
let conn_from_id = conn_from.id();
let conn_to_id = conn_to.id();

// Build the node
let node = Node::sync::<V2>()
    .connection(Network::sync()
        .add_connection(conn_from)
        .add_connection(conn_to)
    ).build().unwrap();

for event in node.events() {
    match event {
        // When frame is received from the "from" connection
        Event::Frame(frame, callback) if callback.connection_id() == conn_from_id => {
            // Broadcast it to the "to" connection
            node.broadcast_frame(
                &frame,
                BroadcastScope::ExactConnection(conn_to_id)
            ).unwrap()
        }
        /* handle other events */
        # _ => {}
    }
}
```

In this particular case it may be ok to simply create two nodes. However, in more elaborated
scenarios you may either have more connections or interested in a far more granular control over
routing. For example, you may route frames based on their system `ID`.

You can learn more about networks and routing in the
[Networks and Routing](crate::docs::b4__networks_and_routing) section.

## Dialects

Maviola both packages canonical MAVLink and provides a way to define your own dialects. Check
[Dialect Constraints](crate::docs::b1__dialect_constraints) to learn how to specify dialects. This
section contains only a brief introduction and links to the related sections of documentation.

### Canonical Dialects

Standard MAVLink dialect can be enabled by the corresponding feature flags. The [`Minimal`] dialect
is required for proper work of the library and always enabled.

These MAVLink dialects are re-exported by [Mavio](https://gitlab.com/mavka/libs/mavio) and
available in [`dialects`] module:

* [`minimal`](https://mavlink.io/en/messages/minimal.html) — minimal dialect required to
   expose your presence to other MAVLink devices (this dialect is enabled by default).
* [`standard`](https://mavlink.io/en/messages/standard.html) — a superset of `minimal` dialect,
   that expected to be used by almost all flight stack.
* [`common`](https://mavlink.io/en/messages/common.html) — minimum viable dialect with most of
   the features, a building block for other future-rich dialects.
* [`ardupilotmega`](https://mavlink.io/en/messages/common.html) — feature-full dialect used by
   [ArduPilot](http://ardupilot.org). In most cases this dialect is the go-to choice if you want
   to recognize almost all MAVLink messages used by existing flight stacks.
* [`all`](https://mavlink.io/en/messages/all.html) — meta-dialect which includes all other
   standard dialects including these which were created for testing purposes. It is guaranteed
   that namespaces of the dialects in `all` family do not collide.
* Other dialects from MAVLink XML [definitions](https://github.com/mavlink/mavlink/tree/master/message_definitions/v1.0):
   `asluav`, `avssuas`, `csairlink`, `cubepilot`, `development`, `icarous`, `matrixpilot`,
   `paparazzi`, `ualberta`, `uavionix`. These do not include `python_array_test` and `test`
   dialects which should be either generated manually or as a part of `all` meta-dialect.

### Default Dialect

There is a "main sequence" of canonical dialects ordered by inclusion:

[`minimal`](https://mavlink.io/en/messages/minimal.html) <
[`standard`](https://mavlink.io/en/messages/standard.html) <
[`common`](https://mavlink.io/en/messages/common.html) <
[`ardupilotmega`](https://mavlink.io/en/messages/common.html) <
[`all`](https://mavlink.io/en/messages/all.html)

Maviola will define [`DefaultDialect`] based on the most general available dialect. The default
dialect is used in all situations, when dialect is assumed but library client hasn't specified it.

### Custom Dialects

Custom MAVLink dialects can be generated from XML message definitions using
[MAVSpec](https://gitlab.com/mavka/libs/mavspec). Check
[Custom Dialects](crate::docs::c1__custom_dialects) section of this documentation for details.

### Ad-hoc Dialects

It is possible to define MAVLink dialects purely by Rust code without creating XML definitions. This
is useful, when experimenting with custom messages or defining messages that makes sense only for
your own project. Check [Ad-hoc Dialects](crate::docs::c4__ad_hoc_dialects) section of this
documentation for details.

<em>[← Quickstart](crate::docs::a1__quickstart) | [Synchronous API →](crate::docs::a3__sync_api)</em>

[`dialects`]: crate::protocol::dialects
[`Minimal`]: crate::protocol::dialects::Minimal
[`protocol`]: crate::protocol
[`message_id`]: crate::protocol::Frame::message_id
[`prelude`]: crate::prelude
[`sync`]: crate::sync
[`sync::Event`]: crate::sync::node::Event
[`sync::prelude`]: crate::sync::prelude
[`asnc`]: crate::asnc
[`asnc::Event`]: crate::asnc::node::Event
[`asnc::prelude`]: crate::asnc::prelude
 */

#[cfg(doc)]
use crate::core::marker::*;
#[cfg(doc)]
use crate::prelude::*;
#[cfg(doc)]
use crate::protocol::*;
