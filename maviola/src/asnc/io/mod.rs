//! # Asynchronous I/O primitives
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

mod callback;
mod channel;
mod connection;
mod transport;
mod types;

pub use callback::Callback;

/// <sup>`⍚` |</sup>
#[cfg(feature = "unstable")]
pub use channel::{Channel, ChannelFactory};
/// <sup>`⍚` |</sup>
#[cfg(feature = "unstable")]
pub use connection::{Connection, ConnectionBuilder};
/// <sup>`⍚` |</sup>
#[cfg(feature = "unstable")]
pub use types::{FrameProducer, FrameReceiver, FrameSendHandler, FrameSender};

#[cfg(not(feature = "unstable"))]
pub(crate) use channel::{Channel, ChannelFactory};
#[cfg(not(feature = "unstable"))]
pub(crate) use connection::{Connection, ConnectionBuilder};
#[cfg(not(feature = "unstable"))]
pub(crate) use types::{FrameProducer, FrameReceiver, FrameSendHandler, FrameSender};

pub(crate) use connection::{ConnReceiver, ConnSender};
