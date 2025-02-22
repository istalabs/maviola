pub mod smalltalk {
    //! # SmallTalk Ad-hoc Dialect
    //!
    //! This is a simple ad-hoc dialect that can be used in doc tests to showcase ad-hoc dialect
    //! construction.
    //!
    //! # Examples
    //!
    //! ```rust,no_run
    //! use maviola::test_utils::smalltalk::*;
    //!
    //! let message = SmallTalk::Howdy(Howdy{
    //!     mood: Mood::Delighted
    //! });
    //! ```

    use crate::protocol::derive::{Dialect, Enum, Message};
    use crate::protocol::dialects::minimal::messages::Heartbeat;
    use crate::protocol::mavspec;

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
}
