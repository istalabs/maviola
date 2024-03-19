//! # Synchronous I/O primitives
//!
//! ## Connections & Channels
//!
//! > ⚠ This part of the API allows to create custom transports. It is still considered experimental
//! > and available only under the `unstable` feature (such entities are marked with <sup>`⍚`</sup>).
//!
//! I/O is based on two main abstraction: connections and channels. [`Connection`] represents an
//! interface to an underlying transport, while [`Channel`] is an individual stream withing a
//! connection. Connections are created by implementors of [`ConnectionBuilder`] trait and channels
//! are constructed by [`ChannelFactory`] which is bounded to a particular connection.
//!
//! In most cases channels and connections are hidden to library user. Dealing with these
//! abstractions is necessary only to those who are interested in creating custom connections.

mod bus;
mod channel;
mod connection;
mod transport;

pub(super) use bus::{incoming_channel, outgoing_channel};

/// <sup>`⍚` |</sup>
#[cfg(feature = "unstable")]
pub use bus::{
    IncomingFrameProducer, IncomingFrameReceiver, OutgoingFrameHandler, OutgoingFrameSender,
};
/// <sup>`⍚` |</sup>
#[cfg(feature = "unstable")]
pub use channel::{Channel, ChannelFactory};
/// <sup>`⍚` |</sup>
#[cfg(feature = "unstable")]
pub use connection::{Connection, ConnectionBuilder, ConnectionHandler};

#[cfg(not(feature = "unstable"))]
pub(in crate::sync) use bus::{
    IncomingFrameProducer, IncomingFrameReceiver, OutgoingFrameHandler, OutgoingFrameSender,
};
#[cfg(not(feature = "unstable"))]
pub(in crate::sync) use channel::ChannelFactory;
#[cfg(not(feature = "unstable"))]
pub(in crate::sync) use connection::{Connection, ConnectionBuilder, ConnectionHandler};
