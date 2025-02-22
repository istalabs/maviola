/*!
# 📖 3.4. Ad-hoc Dialects

<em>[← Custom Processing](crate::docs::c3__custom_processing) | [Testing →](crate::docs::e3__testing)</em>

While in [Custom Dialects](crate::docs::c1__custom_dialects) we've explained how to generate custom
dialects from XML definitions, this chapter shows how one can create ad-hoc dialects using pure
Rust.

We call these dialects ad-hoc because they can be used only inside
[Mavka](https://mavka.gitlab.io/home/) toolchain and other tools are unable to process them.
That doesn't mean these dialects are useless. Quite opposite, by dedicating a particular segment of
message `ID` namespace to ad-hoc dialects, you may achieve interesting results.

## What is Small Talk?

Let's define a simple "small_talk" dialect:

```rust,no_run
use maviola::protocol::dialects::minimal::messages::Heartbeat;
use maviola::protocol::derive::{Dialect, Enum, Message};
use maviola::protocol::mavspec;

/// Communication mood.
#[repr(u8)]
#[derive(Copy, Clone, Debug, Default, Enum)]
pub enum Mood {
    /// Speaks politely.
    #[default]
    Polite = 0,
    /// Dead serious.
    Serious = 1,
    /// Not in the mood.
    Grumpy = 2,
    /// Delighted on the verge of delusion.
    Delighted = 3,
    /// Confused and distracted.
    Confused = 4,
}

/// "How are you?" rhetorical question.
///
/// Well, maybe not always rhetorical.
#[derive(Clone, Debug, Message)]
#[message_id(72000)]
pub struct Howdy {
    /// Communication mood.
    #[base_type(u8)]
    pub mood: Mood,
}

/// I'm good, fine.
#[derive(Clone, Debug, Message)]
#[message_id(72001)]
pub struct Good {
    /// Communication mood.
    #[base_type(u8)]
    pub mood: Mood,
}

/// Returns the previous question
#[derive(Clone, Debug, Message)]
#[message_id(72002)]
pub struct AndYou {
    /// Communication mood.
    #[base_type(u8)]
    pub mood: Mood,
}

/// A strange claim.
#[derive(Clone, Debug, Message)]
#[message_id(72003)]
pub struct NonSequitur {
    /// Communication mood.
    #[base_type(u8)]
    pub mood: Mood,
}

/// Returned on confusion.
#[derive(Clone, Debug, Message)]
#[message_id(72004)]
pub struct Wat {
    /// Communication mood.
    #[base_type(u8)]
    pub mood: Mood,
}

/// SmallTalk Ad-hoc Dialect.
#[derive(Dialect)]
#[dialect(1099)]
#[version(99)]
pub enum SmallTalk {
    /// We want to be compatible with heartbeat protocol.
    Heartbeat(Heartbeat),
    /// — How are you?
    Howdy(Howdy),
    /// — Good.
    Good(Good),
    /// — And you?
    AndYou(AndYou),
    /// — I don't have a precise answer to your question. It was raining this morning. A black
    ///   terrier jumped over a bench scaring a flock of birds. My coffee becomes cold as I was
    ///   watching the clouds changing their elusive shape. When suddenly...
    NonSequitur(NonSequitur),
    /// — WAT!!!
    Wat(Wat),
}
```

Looks okay for our purposes. If you want to know, how to build more complex messages, refer to
[MAVSpec](https://gitlab.com/mavka/libs/mavspec) documentation.

## Let's Have A Small Talk

Now, it is time to use our bespoke dialect. But this as simple, as using canonical dialects:

```rust,no_run
use maviola::prelude::*;
use maviola::sync::prelude::*;
use maviola::test_utils::smalltalk::*;

let node = Node::sync::<V2>()
    .id(MavLinkId::new(1, 17))
    .dialect::<SmallTalk>()
    /* other node setting */
    # .connection(TcpClient::new("127.0.0.1:5600").unwrap())
    .build().unwrap();

node.send(&NonSequitur{
    mood: Mood::Confused,
}).unwrap();
```

As you can see, we use [`maviola::test_utils`] module. This module contains utilities for testing.
To use it, you need to enable `test_utils` Cargo feature. Not the best choice for production but
can be useful for writing your own documentation.

<em>[← Custom Processing](crate::docs::c3__custom_processing) | [Guidelines →](crate::docs::e1__guidelines)</em>

[`maviola::test_utils`]: crate::test_utils
 */
