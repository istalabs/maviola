/*!
# 📖 1.1. Quickstart

<em>[← Home](crate::docs) | [Overview →](crate::docs::a2__overview)</em>

This section provides a simple synchronous example. If you are interested in asynchronous API, check
the [Asynchronous API](crate::docs::a4__async_api) section.

## Install

Add Maviola to your project dependencies:

```shell
cargo add maviola --features sync,common
```

Here, we are enabling synchronous API and [`common`](https://mavlink.io/en/messages/common.html)
MAVLink dialect.

## A Glimpse

Let's start from a simple example:

```rust,no_run
use maviola::test_utils;
use maviola::prelude::*;
use maviola::sync::prelude::*;
# fn main() -> Result<()> {
#
let node = Node::sync::<V2>()
    .id(MavLinkId::new(1, 17))
    .connection(TcpServer::new("127.0.0.1:5600")?)
    .build()?;

for event in node.events() {
    match event {
        Event::NewPeer(peer) => {
            println!("New MAVLink device joined the network: {:?}", peer);
        }
        Event::PeerLost(peer) => {
            println!("MAVLink device is no longer active: {:?}", peer);
        }
        Event::Frame(frame, callback) => {
            if let Ok(message) = frame.decode::<DefaultDialect>() {
                println!(
                    "Received a message from {}:{}: {:?}",
                    frame.system_id(), frame.component_id(), message
                );
                callback.broadcast(&frame)?;
            }
        }
        _ => {}
    }
}
#
# Ok(()) }
```

In the next sections we will explain each step. Shall we start?

## Creating a Node

Let's create a TCP server listening to `5600` port on the local host.

First, we are going import [`prelude`] and [`sync::prelude`]. The former provides common imports
for all kind of API, while the latter imports additional traits required for working with
synchronous API.

```rust
use maviola::prelude::*;
use maviola::sync::prelude::*;
```

Then create a TCP server connection:

```rust,no_run
# use maviola::prelude::*;
# use maviola::sync::prelude::*;
#
let connection = TcpServer::new("127.0.0.1:5600").unwrap();

println!("Connection info: {:?}", connection.info());
```

Other available transports are:

* [`TcpServer`] / [`TcpClient`]
* [`UdpServer`] / [`UdpClient`]
* [`SockServer`] / [`SockClient`] (Unix-like systems only)
* [`FileWriter`] / [`FileReader`]

You can learn, how to create your own transports in
[Custom Transport](crate::docs::c2__custom_transport) section of this documentation.

Once we have a connection, we can create a MAVLink [`Node`] with specified [`SystemId`] and
[`ComponentId`]:

```rust,no_run
# use maviola::prelude::*;
# use maviola::sync::prelude::*;
# let connection = TcpServer::new("127.0.0.1:5600").unwrap();
#
let node = Node::sync::<V2>()
    .system_id(1)
    .component_id(17)
    .connection(connection)
    .build().unwrap();
```

Such nodes called [`Edge`]. That means, that these nodes represent a particular MAVLink device
within a network. These types of nodes can send heartbeats and perform other automatic operations.

You can also use `id`, that accepts [`MavLinkId`] object with specified system and component `ID`s
instead of setting them separately:

```rust,no_run
# use maviola::prelude::*;
# use maviola::sync::prelude::*;
#
let node = Node::sync::<V2>()
    .id(MavLinkId::new(1, 17))
    // ...
# ;
```

Let's activate our node:

```rust,no_run
# use maviola::prelude::*;
# use maviola::sync::prelude::*;
# let node = Node::sync::<V2>()
#    .id(MavLinkId::new(1, 17))
#    .connection(TcpServer::new("127.0.0.1:5600").unwrap())
#    .build().unwrap();
#
let mut node: EdgeNode<V2> = node;
node.activate().unwrap();
```

As you can see, we've changed our node to mutable and used [`EdgeNode`] as a type alias. This is
because activation of a node requires mutable access. Also, it is always easier to use [`EdgeNode`]
instead of [`Node<Edge<_>, _, SyncApi<_>>`], especially when you define a function, that accepts or
returns a node. We don't need this at the moment, but it will come handy later.

You've probably mentioned, that we use [`V2`] as a type parameter for our node. This is because
we want our node to speak `MAVLink 2` protocol version. By default, nodes are [`Versionless`], which
means that they can send and receive both `MAVLink 1` and `MAVLink 2` frames. However, versionless
nodes can't send automatic heartbeats (what protocol version they should use?) and are generally a
bit more awkward to use.

## Receive Frames

It is time to use out node! Let's subscribe to [`events`] and monitor incoming heartbeats:

```rust,no_run
# use maviola::prelude::*;
# use maviola::sync::prelude::*;
# let node = Node::sync::<V2>()
#    .id(MavLinkId::new(1, 17))
#    .connection(TcpServer::new("127.0.0.1:5600").unwrap())
#    .build().unwrap();
#
for event in node.events() {
    if let Event::Frame(frame, _) = event {
        if let Ok(DefaultDialect::Heartbeat(msg)) = frame.decode() {
            println!(
                "Incoming heartbeat from {}:{}: {:?}",
                frame.system_id(), frame.component_id(), msg
            )
        }
    }
}
```

## Respond to Frames

Once frame is receive, we may want to respond to its sender or broadcast it to other MAVLink nodes
in our network. Let's respond to a heartbeat with a heartbeat:

```rust,no_run
# use maviola::prelude::*;
# use maviola::sync::prelude::*;
# let node = Node::sync::<V2>()
#    .id(MavLinkId::new(1, 17))
#    .connection(TcpServer::new("127.0.0.1:5600").unwrap())
#    .build().unwrap();
#
use maviola::protocol::dialects::minimal::messages::Heartbeat;

for event in node.events() {
    if let Event::Frame(frame, callback) = event {
        if let Ok(DefaultDialect::Heartbeat(msg)) = frame.decode() {
            println!(
                "Incoming heartbeat from {}:{}: {:?}",
                frame.system_id(), frame.component_id(), msg
            );
            let response_frame = node.next_frame(&Heartbeat::default()).unwrap();
            callback.respond(&response_frame).unwrap()
        }
    }
}
```

Here, we've asked our node to create a frame from default heartbeat message. This frame will contain
our node's system and component `ID`s and a proper MAVLink frame sequence. We can also use
[`broadcast`] instead of [`respond`] to send incoming frame to other members of a network:

```rust,no_run
# use maviola::prelude::*;
# use maviola::sync::prelude::*;
# let node = Node::sync::<V2>()
#    .id(MavLinkId::new(1, 17))
#    .connection(TcpServer::new("127.0.0.1:5600").unwrap())
#    .build().unwrap();
#
use maviola::protocol::dialects::minimal::messages::Heartbeat;

for event in node.events() {
    if let Event::Frame(frame, callback) = event {
        if let Ok(DefaultDialect::Heartbeat(msg)) = frame.decode() {
            println!(
                "Incoming heartbeat from {}:{}: {:?}",
                frame.system_id(), frame.component_id(), msg
            );
            callback.broadcast(&frame).unwrap()
        }
    }
}
```

## Send a Frame

It is useful to respond to incoming frames. But what if we want to send frames proactively? We can
use our node's [`send`] method:

```rust,no_run
# use maviola::prelude::*;
# use maviola::sync::prelude::*;
# let node = Node::sync::<V2>()
#    .id(MavLinkId::new(1, 17))
#    .connection(TcpServer::new("127.0.0.1:5600").unwrap())
#    .build().unwrap();
#
use maviola::protocol::dialects::minimal::messages::Heartbeat;
use maviola::protocol::dialects::minimal::enums;

let msg = Heartbeat {
    type_: enums::MavType::FixedWing,
    autopilot: enums::MavAutopilot::GenericMissionFull,
    mavlink_version: DefaultDialect::version().unwrap(),
    ..Default::default()
};

node.send(&msg).unwrap();
```

## Error Handling

Maviola provides its own root [`Error`] and [`Result`]. All other errors returned by methods and
functions in this library can be converted to the root error. Which means that we can use something
like:

```rust,no_run
use maviola::prelude::*;
use maviola::sync::prelude::*;

fn new() -> Result<()> {
    let node = Node::sync::<V2>()
        .id(MavLinkId::new(1, 17))
        .connection(TcpServer::new("127.0.0.1:5600")?)
        .build()?;
    /* some interesting work */
    Ok(())
}
```

Way better (and safer).

In the next sections of this documentation we will often assume, that complex code examples are
executed in the context of a function, that returns [`Result`] (for simple examples we will keep
unwrapping stuff for clarity).

## Congratulations

That's it! Now you can move to the next sections of the [documentation](crate::docs). First of all,
check the [Overview](crate::docs::a2__overview) section to learn how Maviola is organized.

<em>[← Home](crate::docs) | [Overview →](crate::docs::a2__overview)</em>

[`prelude`]: crate::prelude
[`sync::prelude`]: crate::sync::prelude
[`events`]: crate::sync::node::ReceiveEvent::events
[`broadcast`]: Callback::broadcast
[`respond`]: Callback::respond
[`send`]: Node::send
 */

#[cfg(doc)]
use crate::core::marker::*;
#[cfg(doc)]
use crate::prelude::*;
#[cfg(doc)]
use crate::protocol::*;
#[cfg(doc)]
#[cfg(feature = "sync")]
use crate::sync::prelude::*;
