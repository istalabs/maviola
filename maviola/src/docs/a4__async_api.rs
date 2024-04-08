/*!
# 📖 1.4. Asynchronous API

<em>[← Synchronous API](crate::docs::a3__sync_api) | [Dialect Constraints →](crate::docs::b1__dialect_constraints)</em>

If you've read [Overview](crate::docs::a2__overview), you may already familiarize yourself
with the basic API abstraction. This chapter will describe them in depth focusing on the specifics of
asynchronous API.

In any case, we suggest you at least to check [Choosing Your API](crate::docs::a2__overview#choosing-your-api)
before reading this.

## Contents

1. [Install](#install)
1. [Basics](#basics)
1. [Receiving](#receiving)
1. [Sending](#sending)
    1. [Sending Frames](#sending-frames)
    1. [Proxy Nodes & Devices](#proxy-nodes--devices)
    1. [Dependent Nodes](#dependent-nodes)
1. [Handling Peers](#handling-peers)
1. [Active Nodes & Heartbeats](#active-nodes--heartbeats)
1. [Multitasking](#multitasking)

## Install

To use synchronous API we have to install Maviola with `async` feature flag.

```shell
cargo add maviola --features async
```

## Basics

Let's catch up with the example from the [Quickstart](crate::docs::a1__quickstart) chapter, but
this time adjusted to asynchronous API:

```rust,no_run
use maviola::prelude::*;
use maviola::asnc::prelude::*;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let node = Node::asnc::<V2>()
        .id(MavLinkId::new(1, 17))
        .connection(TcpServer::new("127.0.0.1:5600").unwrap())
        .build().await.unwrap();

    let mut events = node.events().unwrap();
    while let Some(event) = events.next().await {
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
                    callback.broadcast(&frame).unwrap();
                }
            }
            _ => {}
        }
    }
}
```

Here we've created a [`Node`] with `system_id=1` and `component_id=17` that serves as a TCP server
bound to `127.0.0.1:5600`. Then we subscribe to node `events`, intercepting incoming frames and
broadcasting them to all TCP clients except those that sent the original frame.

ⓘ As you've probably noticed we are using a `#[tokio::main]` helper attribute from
[Tokio](https://tokio.rs/) to set up asynchronous runtime by decorating the `main` function. We are
going to omit this setup in all further examples to avoid cluttering. It's also worth mentioning,
that since we are using asynchronous [streams](https://tokio.rs/tokio/tutorial/streams), it is
necessary to import [`tokio_stream::StreamExt`]. In this case [`asnc::prelude`] takes care about
importing all necessary traits.

ⓘ Asynchronous API lives in the [`asnc`] module. You can always check its documentation for the
specifics.

With all this in mind, let's dig into the details!

## Receiving

As we've mentioned early, the `events` method is the suggested approach for dealing with everything
that node receives. You can check the documentation for [`Event`] to learn more about available
events.

ⓘ To access `events` we need to import [`ReceiveEvent`] trait. We don't do this explicitly since we
use [`asnc::prelude`].

Still, if we are not interested in monitoring peers we can subscribe to `frames` directly. This
method that returns an iterator over valid incoming frames:

```rust,no_run
# #[tokio::main(flavor = "current_thread")] async fn main () {
# use maviola::prelude::*;
# use maviola::asnc::prelude::*;
# let node = Node::asnc::<V2>()
#    .id(MavLinkId::new(1, 17))
#    .connection(TcpServer::new("127.0.0.1:5600").unwrap())
#    .build().await.unwrap();
#
let mut frames = node.frames().unwrap();
while let Some((frame, callback)) = frames.next().await {
    if let Ok(message) = frame.decode::<DefaultDialect>() {
        println!(
            "Received a message from {}:{}: {:?}",
            frame.system_id(), frame.component_id(), message
        );
        callback.broadcast(&frame).unwrap();
    }
}
# }
```

ⓘ Working on the frame level requires importing [`ReceiveFrame`] trait. Once again,
[`asnc::prelude`] can do it for us.

We are not bound to use iterators. In some cases you might be interested in receiving just the next
[`Event`] or [`Frame`]. For example:

```rust,no_run
# #[tokio::main(flavor = "current_thread")] async fn main () {
# use maviola::prelude::*;
# use maviola::asnc::prelude::*;
# let mut node = Node::asnc::<V2>()
#    .id(MavLinkId::new(1, 17))
#    .connection(TcpServer::new("127.0.0.1:5600").unwrap())
#    .build().await.unwrap();
#
let next_event = node.recv().await.unwrap();
# }
```

Or in case of the frame:

```rust,no_run
# #[tokio::main(flavor = "current_thread")] async fn main () {
# use maviola::prelude::*;
# use maviola::asnc::prelude::*;
# let mut node = Node::asnc::<V2>()
#    .id(MavLinkId::new(1, 17))
#    .connection(TcpServer::new("127.0.0.1:5600").unwrap())
#    .build().await.unwrap();
#
let next_frame = node.recv_frame().await.unwrap();
# }
```

The interface for receiving events and frames is very similar to [`std::sync::mpsc::Receiver`].
The difference is that we return our own set of errors:

* [`RecvError`] for [`recv`] and [`recv_frame`]
* [`RecvTimeoutError`] for [`recv_timeout`] and [`recv_frame_timeout`]
* [`TryRecvError`] for [`try_recv`] and [`try_recv_frame`]

Another important difference is that we may have multiple receivers for the same channel as
explained in the [Multithreading](#multithreading) below.

## Sending

We've already learned how to respond to a frame using `callback`. We suggest to check [`Callback`]
documentation to learn more about all available methods.

ⓘ Working with [`Callback`] requires importing [`CallbackApi`] trait. The other reason to use
[`prelude`] that imports it for us.

If we want to send messages proactively, then need to use node's sending API:

```rust,no_run
# #[tokio::main(flavor = "current_thread")] async fn main () {
# use maviola::prelude::*;
# use maviola::asnc::prelude::*;
# let node = Node::asnc::<V2>()
#    .id(MavLinkId::new(1, 17))
#    .connection(TcpServer::new("127.0.0.1:5600").unwrap())
#    .build().await.unwrap();
#
let message = default_dialect::messages::Heartbeat::default();
node.send(&message).unwrap();
# }
```

### Sending Frames

This covers most of the cases. However, sometimes we may want to send frame directly instead of
message. In such case we need a `send_frame` method:

```rust,no_run
# #[tokio::main(flavor = "current_thread")] async fn main () {
# use maviola::prelude::*;
# use maviola::asnc::prelude::*;
# let node = Node::asnc::<V2>()
#    .id(MavLinkId::new(1, 17))
#    .connection(TcpServer::new("127.0.0.1:5600").unwrap())
#    .build().await.unwrap();
#
let message = default_dialect::messages::Heartbeat::default();
let frame = node.next_frame(&message).unwrap();
node.send_frame(&frame).unwrap();
# }
```

ⓘ To send frames we need to import [`SendFrame`] trait. Sending messages requires additional
[`SendMessage`] trait to be imported as well. Both of these traits are available in [`prelude`].

### Proxy Nodes & Devices

The above approach works only for edge nodes (i.e. [`EdgeNode`]). If we are dealing with a
[`ProxyNode`], then we need to use different approach. We need to create a [`Device`] with specified
system and component `ID`s:

```rust,no_run
# #[tokio::main(flavor = "current_thread")] async fn main () {
# use maviola::prelude::*;
# use maviola::asnc::prelude::*;
# let node = Node::asnc::<V2>()
#    .connection(TcpServer::new("127.0.0.1:5600").unwrap())
#    .build().await.unwrap();
#
let device = Device::new(MavLinkId::new(2, 42), &node);
# }
```

Then we can create and send frames in the same fashion:

```rust,no_run
# #[tokio::main(flavor = "current_thread")] async fn main () {
# use maviola::prelude::*;
# use maviola::asnc::prelude::*;
# let node = Node::asnc::<V2>()
#    .connection(TcpServer::new("127.0.0.1:5600").unwrap())
#    .build().await.unwrap();
#
# let device = Device::new(MavLinkId::new(2, 42), &node);
let message = default_dialect::messages::Heartbeat::default();
let frame = device.next_frame(&message).unwrap();
node.send_frame(&frame).unwrap();
# }
```

**⚠** It is important to remember, that if you communicate on behalf of a device, MAVLink
specification requires you to send heartbeats. In Maviola only edge nodes can do that automatically
as described in [Active Nodes & Heartbeats](#active-nodes--heartbeats). In the case of devices you
have to send heartbeats manually or use [dependent nodes](#dependent-nodes).

### Dependent Nodes

While [`Device`] abstraction is useful ang gives a fine-grained control over frame processing, in
most cases it would be advantageous to reuse a connection of an existing node for the new one. Such
nodes are called "dependent" nodes and can be built using node builder:

```rust,no_run
# #[tokio::main(flavor = "current_thread")] async fn main () {
# use maviola::prelude::*;
# use maviola::asnc::prelude::*;
#
let proxy_node = Node::asnc::<V2>()
    .connection(TcpServer::new("127.0.0.1:5600").unwrap())
    /* we can add frame processing settings here */
    .build().await.unwrap();

let mut edge_node = Node::asnc()
    .id(MavLinkId::new(1, 17))
    /* other node settings that do not include connection */
    .build_from(&proxy_node);
# }
```

Such nodes can be created only from a [`ProxyNode`] and are always [`EdgeNode`]s. They will use
[`FrameProcessor::compat`] and [`FrameProcessor::signer`] from a "parent" node if these parameters
hasn't been set explicitly for the "dependent" node. They will also add all known dialects from the
parent edge node and all [custom processors](crate::docs::c3__custom_processing).

## Handling Peers

As you've probably seen, we have special events [`Event::NewPeer`] and [`Event::PeerLost`]. These
events are signaling that a certain peer sent their first heartbeat or certain peer hasn't been
seen for a while. Peers are distinguished purely by their system and component `ID`s.

The duration after which peer will be considered lost is defined by [`Node::heartbeat_timeout`]
the default value is [`DEFAULT_HEARTBEAT_TIMEOUT`]. You can set this value when building a node:

```rust,no_run
# use maviola::prelude::*;
# use maviola::asnc::prelude::*;
use std::time::Duration;

Node::asnc::<V2>()
    .heartbeat_timeout(Duration::from_millis(500))
    /* other node settings */
# ;
```

## Active Nodes & Heartbeats

It's nice to receive heartbeats. But what about sending them? Simple. Let's first create an edge
node:

```rust,no_run
# #[tokio::main(flavor = "current_thread")] async fn main () {
# use maviola::prelude::*;
# use maviola::asnc::prelude::*;
#
let mut node = Node::asnc::<V2>()
    .id(MavLinkId::new(1, 17))
    .connection(TcpServer::new("127.0.0.1:5600").unwrap())
    .build().await.unwrap();
# }
```

Then activate it:

```rust,no_run
# #[tokio::main(flavor = "current_thread")] async fn main () {
# use maviola::prelude::*;
# use maviola::asnc::prelude::*;
# let mut node = Node::asnc::<V2>()
#    .id(MavLinkId::new(1, 17))
#    .connection(TcpServer::new("127.0.0.1:5600").unwrap())
#    .build().await.unwrap();
#
node.activate().await.unwrap();
# }
```

This will transition node into active mode, and it will start to send automatic heartbeats
immediately. The default heartbeat interval is defined by [`DEFAULT_HEARTBEAT_INTERVAL`] constant.
You can change it during node construction:

```rust,no_run
# use maviola::prelude::*;
# use maviola::asnc::prelude::*;
use std::time::Duration;

Node::asnc::<V2>()
    .id(MavLinkId::new(1, 17))
    .heartbeat_interval(Duration::from_millis(500))
    /* other node settings */
# ;
```

Finally, you can deactivate active node to prevent it from sending heartbeats by calling
[`Node::deactivate`].

You can check whether node is active by calling [`Node::is_active`].

## Multitasking

Nodes handle connections and therefore are neither [`Sync`] nor [`Send`]. You obviously may
wrap them with [`Arc`] or even arc-mutex them but this not always what you want. First, mutexes
are not just heavy, they also not always convenient. And in the case of the [`Arc`] you can't
explicitly drop the node since some nasty function may still hold a reference to it.

To solve this problem, we provide [`Node::sender`] and [`Node::receiver_mut`] /
[`Node::receiver_cloned`] methods that return sending and receiving parts of a node.

To send frames in an asynchronous task obtain a [`FrameSender`] that implements [`SendFrame`] and
[`SendMessage`] traits and use it in the same way you are using node:

```rust,no_run
# #[tokio::main(flavor = "current_thread")] async fn main () {
# use maviola::prelude::*;
# use maviola::asnc::prelude::*;
# let mut node = Node::asnc::<V2>()
#    .id(MavLinkId::new(1, 17))
#    .connection(TcpServer::new("127.0.0.1:5600").unwrap())
#    .build().await.unwrap();
#
use tokio::task;

let sender = node.sender();

task::spawn(async move {
    sender.send(
        &default_dialect::messages::Heartbeat::default()
    ).unwrap();
}).await.unwrap();
# }
```

If instead you want to receive frames or events "out there", obtain a [`EventReceiver`] that
implements [`ReceiveEvent`] and [`ReceiveFrame`] traits and use it freely:

```rust,no_run
# #[tokio::main(flavor = "current_thread")] async fn main () {
# use maviola::prelude::*;
# use maviola::asnc::prelude::*;
# let mut node = Node::asnc::<V2>()
#    .id(MavLinkId::new(1, 17))
#    .connection(TcpServer::new("127.0.0.1:5600").unwrap())
#    .build().await.unwrap();
#
use tokio::task;

let receiver = node.receiver_cloned();

task::spawn(async move {
    let mut frames = receiver.frames().unwrap();
    while let Some((frame, callback)) = frames.next().await {
        callback.send(&frame).unwrap();
    }
}).await.unwrap();
# }
```

And, yes, you can respond to frames from a receiver using `callback`.

ⓘ The interesting difference between [`Node::sender`] and [`Node::receiver_mut`] is that the latter
returns a mutable reference instead of a new object (to obtain a new cloned object use
[`Node::receiver_cloned`]). Which means that you may gain some performance improvement by refraining
from cloning it. This also makes sense since receivers have access only to events that were emitted
after their creation. This is related to the limitations of the underlying MPMC channel.

<em>[← Synchronous API](crate::docs::a3__sync_api) | [Dialect Constraints →](crate::docs::b1__dialect_constraints)</em>

[`Arc`]: std::sync::Arc
[`prelude`]: crate::prelude
[`asnc`]: crate::asnc
[`asnc::prelude`]: crate::asnc::prelude
[`recv`]: crate::asnc::node::ReceiveEvent::recv
[`recv_timeout`]: crate::asnc::node::ReceiveEvent::recv_timeout
[`try_recv`]: crate::asnc::node::ReceiveEvent::recv
[`recv_frame`]: crate::asnc::node::ReceiveFrame::recv_frame
[`recv_frame_timeout`]: crate::asnc::node::ReceiveFrame::recv_frame_timeout
[`try_recv_frame`]: crate::asnc::node::ReceiveFrame::recv_frame
[`DEFAULT_HEARTBEAT_TIMEOUT`]: crate::core::consts::DEFAULT_HEARTBEAT_TIMEOUT
[`DEFAULT_HEARTBEAT_INTERVAL`]: crate::core::consts::DEFAULT_HEARTBEAT_INTERVAL
 */

#[cfg(doc)]
use crate::asnc::prelude::*;
#[cfg(doc)]
use crate::core::marker::*;
#[cfg(doc)]
use crate::error::*;
#[cfg(doc)]
use crate::prelude::*;
#[cfg(doc)]
use crate::protocol::*;
