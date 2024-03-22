/*!
# 📖 2.2. Message Signing

Second version of MAVLink protocol adds a capability to
[authenticate](https://mavlink.io/en/guide/message_signing.html) sender by adding a
cryptographic [`signature`] to a frame. When frames are signed, they have a special
[`MAVLINK_IFLAG_SIGNED`] set in their incompatibility flags ([`incompat_flags`]). A receiver can
validate the [`signature`] and either reject unauthorized frame or process it based on the
[`link_id`](Signature::link_id) of the signature.

## Contents

1. [Basics](#basics)
1. [Signing Strategies](#signing-strategies)
1. [Multiple Links](#multiple-links)
1. [Excluding Messages](#excluding-messages)
1. [Unknown Links](#unknown-links)

## Basics

In Maviola we provide a special frame processor called [`FrameSigner`] responsible to signing and
validating frames, when added to a [`Node`]:

```rust,no_run
use maviola::prelude::*;
use maviola::sync::prelude::*;

let node = Node::sync::<V2>()
    .signer(FrameSigner::new(11, "secure key"))
    # .connection(TcpClient::new("127.0.0.1:5600").unwrap())
    /* other node settings */
    .build().unwrap();
```

This will set a [`SecretKey`] for link with `id=11`. This will validate all signed messages and
sign unsigned messages.

## Signing Strategies

Sometimes we need a more nuanced approach to signing. The frame signer can be built with distinct
strategies for [`incoming`] and [`outgoing`] frames. The strategy in each case is defined by
[`SignStrategy`] enum. For example:

```rust,no_run
# use maviola::prelude::*;
# use maviola::sync::prelude::*;
#
let node = Node::sync::<V2>()
    .signer(FrameSigner::builder()
        .link_id(11)
        .key("secret key")
        .incoming(SignStrategy::Strict)
        .outgoing(SignStrategy::Strip)
    )
    # .connection(TcpClient::new("127.0.0.1:5600").unwrap())
    /* other node settings */
    .build().unwrap();
```

This will apply a strict validation for incoming messages and will strip signatures from outgoing
messages.

## Multiple Links

Since `MAVLink 2` protocol supports multiple links, Maviola provides a way to specify additional
links and secret keys. These links with the corresponding keys will be used only for validation.
Frames will always be signed by the main [`key`] and [`link_id`].

```rust,no_run
# use maviola::prelude::*;
# use maviola::sync::prelude::*;
#
let node = Node::sync::<V2>()
    .signer(FrameSigner::builder()
        .link_id(11)
        .key("secret key")
        .add_link(17, "another key")
        .add_link(217, "yet another key")
    )
    # .connection(TcpClient::new("127.0.0.1:5600").unwrap())
    /* other node settings */
    .build().unwrap();
```

## Excluding Messages

You can exclude certain message `ID`s from signature processing by using `exclude` method of a
frame signer builder.

```rust,no_run
# use maviola::prelude::*;
# use maviola::sync::prelude::*;
#
let node = Node::sync::<V2>()
    .signer(FrameSigner::builder()
        .link_id(11)
        .key("secret key")
        .exclude(&[0, 23, 240])
    )
    # .connection(TcpClient::new("127.0.0.1:5600").unwrap())
    /* other node settings */
    .build().unwrap();
```

## Unknown Links

All mentioned rules are strict when applied to frames with unknown link `ID`s. These messages will
be considered always invalid. There is a way to bypass these rules by setting [`unknown_links`]
signing strategy:

```rust,no_run
# use maviola::prelude::*;
# use maviola::sync::prelude::*;
#
let node = Node::sync::<V2>()
    .signer(FrameSigner::builder()
        .link_id(11)
        .key("secret key")
        .unknown_links(SignStrategy::ReSign)
    )
    # .connection(TcpClient::new("127.0.0.1:5600").unwrap())
    /* other node settings */
    .build().unwrap();
```

This will use the main [`key`] to validate frames from unknown links and re-sign them with the main
[`link_id`].

**`⍚`** Unfortunately, this part of API is not stable enough and therefore available only, when
`unstable` Cargo feature is enabled.

<em>[← Dialect Constraints](crate::docs::b1__dialect_constraints) | [Compatibility →](crate::docs::b3__compat_checks)</em>

[`signature`]: Frame::signature
[`incompat_flags`]: Frame::incompat_flags
[`MAVLINK_IFLAG_SIGNED`]: IncompatFlags::MAVLINK_IFLAG_SIGNED
[`incoming`]: FrameSigner::incoming
[`outgoing`]: FrameSigner::outgoing
[`link_id`]: FrameSigner::link_id
[`key`]: FrameSigner::key
[`unknown_links`]: FrameSigner::unknown_links
 */

#[cfg(doc)]
use crate::prelude::*;
#[cfg(doc)]
use crate::protocol::*;
