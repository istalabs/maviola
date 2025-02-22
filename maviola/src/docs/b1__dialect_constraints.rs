/*!
# 📖 2.1. Dialect Constraints

<em>[← Asynchronous API](crate::docs::a4__async_api) | [Message Signing →](crate::docs::b2__signing)</em>

This chapter requires basic understanding of Maviola abstraction. Check
[Quickstart](crate::docs::a1__quickstart) or [Overview](crate::docs::a2__overview) sections before
reading this.

## Contents

1. [Intro](#intro)
1. [Canonical Dialects](#canonical-dialects)
1. [Rules of Dialect Management](#rules-of-dialect-management)
1. [Setting the Main Dialect](#setting-the-main-dialect)
1. [Adding Additional Dialects](#adding-additional-dialects)
1. [Validation](#validation)
1. [Custom Dialects](#custom-dialects)

## Intro

Dialect is a part of MAVLink specification responsible for decoding frames (or packets) to actual
messages. But there is more to it. According to MAVLink packet format, you can't verify the
consistency of a frame without knowing a dialect. This is because [`Frame::checksum`] is calculated
using a `CRC_EXTRA` fingerprint that depends on the message structure.

This decision of protocol designers has both advantages and downsides. The advantage is that once
you've fixed a dialect, you don't need to perform two calculations. One for packet consistency and
another for message integrity. You gain simplicity and performance boost at the same time. The
downside is that if you are building a network infrastructure for all possible dialects, there is
no way to confirm, that packets you are forwarding are correct. Unless you know their dialect
upfront. Which you don't.

There is no workaround for this problem as far as we keep MAVLink packet format untouched. Which
means, that refraining from choosing a dialect (or dialects) has unavoidable risk of polluting
your network infrastructure with junk.

In Maviola we provide instruments for frame validation using a special frame processor called
[`KnownDialects`] that you can attach to a [`Node`].

## Canonical Dialects

In most situations you don't need to worry about all the issues around dialects, extra CRC bytes,
and checksums. You just already know which dialect you are going to speak. If this is one of the
canonical dialect (the dialects you can pick by enabling Cargo feature), you shouldn't do anything
at all.

There is a "main sequence" (yes, you have a right [guess](https://en.wikipedia.org/wiki/Main_sequence))
of canonical dialects ordered by inclusion:

[`minimal`](https://mavlink.io/en/messages/minimal.html) <
[`standard`](https://mavlink.io/en/messages/standard.html) <
[`common`](https://mavlink.io/en/messages/common.html) <
[`ardupilotmega`](https://mavlink.io/en/messages/common.html) <
[`all`](https://mavlink.io/en/messages/all.html)

Maviola will assume the most general available canonical dialect as a [`DefaultDialect`] in all
places which require dialect specification. And that's it. Now, you may simply move to the
[next](crate::docs::b2__signing) chapter.

But what if you want something else? Something special.

## Rules of Dialect Management

ⓘ  First of all, we need to mention that the [`Minimal`] dialect is irreplaceable. You can think
about it as a "rich" empty set or, if you prefer the language of
[abstract nonsense](https://en.wikipedia.org/wiki/Abstract_nonsense), an initial object. It is
always there, it can't be replaced no matter what you are doing. Maviola will hold to it as mother
carries her yet to be born child. Keeping that in mind, let's proceed to dialect management.

Each node has the main [`dialect`] and the list of [`known_dialects`]. The main dialect is used
to communicate dialect capability over MAVLink network using
[heartbeat](https://mavlink.io/en/services/heartbeat.html) protocol. The protocol requires a special
`mavlink_version` field of a
[heartbeat message](https://mavlink.io/en/messages/common.html#HEARTBEAT) to be set from the
[`Dialect::version`].

The known dialects are used to validate incoming and outgoing messages. But the main dialect is
always have precedence over other dialects. That means, that if you have a collision in message
`ID` namespace and your [`Frame::message_id`] belongs to the main dialect, then it will be checked
against this dialect. And if the message fails such validation, it will be rejected. Even if you
have a known dialect for which this frame is valid.

For now, this is the only difference between main and known dialects, but in later versions other
capabilities related to the main dialect may be added to the library.

As mentioned above, the [`Minimal`] dialect is always among the known dialects. But beyond that
you can use whatever you want as a main dialect and any combination of known dialects with one
small restriction, that [`Dialect::name`] can't collide.

## Setting the Main Dialect

When you want your [`Node`] to speak a dialect which is not a [`DefaultDialect`], then you need to
specify this dialect during node construction using `dialect` method and
[turbofish](https://turbo.fish/about) syntax:

```rust,no_run
# use maviola::prelude::*;
# use maviola::sync::prelude::*;
use maviola::protocol::dialects::Minimal;

let node = Node::builder()
    .dialect::<Minimal>()
    /* other node settings */
    # ;
```

Here we replace default dialect with the [`Minimal`] dialect. However, the default dialect will
still be present in the list of the [`known_dialects`]. If you want to completely remove it, use
`no_default_dialect` method of a node builder:

```rust,no_run
# use maviola::prelude::*;
# use maviola::sync::prelude::*;
#
let node = Node::builder()
    .no_default_dialect()
    /* other node settings */
    # ;
```

## Adding Additional Dialects

To add an extra dialect, simply use `add_dialect` of the node builder and our beloved (or bewitched)
[turbofish](https://turbo.fish/about) syntax:

```rust,no_run
# use maviola::prelude::*;
# use maviola::sync::prelude::*;
#
use maviola::protocol::dialects::Minimal;

let node = Node::builder()
    .add_dialect::<Minimal>()
    /* other node settings */
    # ;
```

If your dialect has the same [`Dialect::name`] as one of the already known dialects, then this will
replace a dialect. If for some reason your dialect has a [`Dialect::name`] equal to the main dialect
or [`Minimal`] dialect, then nothing will happen. We do not return error in this case since such
situations are rare, and it takes a significant effort to shoot your leg from such uncomfortable
position (you have to define a custom dialect with a colliding name).

## Validation

Once you've specified (or kept default settings) for your main [`dialect`] and [`known_dialects`],
you can rely on the internal frame validation for nodes. Which means, that all incoming frames
will be checked against your known dialects. The main dialect have a precedence over other dialects
and the oder in which you've added your known dialects matters as well. However, this makes sense
only when you have dialects with colliding message `ID` namespaces. Which doesn't make sense.
Unless you are writing another fundamental MAVLink processing library as we do.

There is one important exception from the validation rules. If [`Frame::message_id`] does not belong
to any of the known dialect, then it will be rejected by default. However, you may change this
behavior by setting `allow_unknown_dialects` in the node builder to `true`:

```rust,no_run
# use maviola::prelude::*;
# use maviola::sync::prelude::*;
#
let node = Node::builder()
    .allow_unknown_dialects(true)
    /* other node settings */
    # ;
```

This may be useful if you perform frame validation after receiving it from the node. Why someone
wants to do that? We don't know. But feel like you should be the one making this decision.

## Custom Dialects

Speaking about peculiar life choices. It is possible to create a custom dialect both from XML
message definition (like canonical MAVLink dialects) and purely from Rust code. The former, most
general, approach is discussed in [Custom Dialects](crate::docs::c1__custom_dialects). The latter,
easier and specific, is explained in [Ad-hoc Dialects](crate::docs::c4__ad_hoc_dialects) chapter.

<em>[← Asynchronous API](crate::docs::a4__async_api) | [Message Signing →](crate::docs::b2__signing)</em>

[`Minimal`]: crate::protocol::dialects::Minimal
[`dialect`]: crate::core::node::Node::dialect
[`known_dialects`]: crate::core::node::Node::known_dialects
 */

#[cfg(doc)]
use crate::prelude::*;
#[cfg(doc)]
use crate::protocol::*;
