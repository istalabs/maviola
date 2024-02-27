//! # AsyncConnection and channels
//!
//! This module contains abstractions for connections and channels. [`AsyncConnection`] represents an
//! interface to an underlying transport, while [`AsyncChannel`] is an individual stream withing a
//! connection. Connections are created by implementors of [`AsyncConnectionBuilder`] trait and channels
//! are constructed by [`AsyncChannelFactory`] which is bounded to a particular connection.
//!
//! In most cases channels and connections are hidden to library user. Dealing with these
//! abstractions is necessary only to those who are interested in creating custom connections.

mod callback;
mod channel;
mod connection;
mod transport;
mod types;

pub use callback::AsyncCallback;
pub use channel::{AsyncChannel, AsyncChannelFactory};
pub use connection::{AsyncConnection, AsyncConnectionBuilder};
pub use transport::{AsyncTcpClient, AsyncTcpServer};
pub use types::{AsyncFrameProducer, AsyncFrameReceiver, AsyncFrameSendHandler, AsyncFrameSender};

pub(crate) use connection::{AsyncConnReceiver, AsyncConnSender};
