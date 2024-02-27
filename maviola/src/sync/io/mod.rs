//! # Synchronous I/O
//!
//! ## Transport
//!
//! The following transports are currently available:
//!
//! * TCP: [`TcpServer`] / [`TcpClient`]
//! * UDP: [`UdpServer`] / [`UdpClient`]
//! * File: [`FileWriter`] / [`FileReader`]
//! * Unix socket: [`SockServer`] / [`SockClient`] (only on Unix-like systems such as Linux or OS X)
//!
//! ## Connections & Channels
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
pub use channel::{Channel, ChannelFactory};
pub use connection::{Connection, ConnectionBuilder};
pub use transport::{FileReader, FileWriter, TcpClient, TcpServer, UdpClient, UdpServer};
#[cfg(unix)]
pub use transport::{SockClient, SockServer};
pub use types::{FrameProducer, FrameReceiver, FrameSendHandler, FrameSender};

pub(crate) use connection::{ConnReceiver, ConnSender};
