//! # Connection and channels
//!
//! This module contains abstractions for connections and channels. [`Connection`] represents an
//! interface to an underlying transport, while [`Channel`] is an individual stream withing a
//! connection. Connections are created by implementors of [`ConnectionBuilder`] trait and channels
//! are constructed by [`ChannelFactory`] which is bounded to a particular connection.
//!
//! In most cases channels and connections are hidden to library user. Dealing with these
//! abstractions is necessary only to those who are interested in creating custom connections.

mod channel;
mod connection;

pub use channel::{Channel, ChannelFactory};
pub use connection::{Connection, ConnectionBuilder};

pub(crate) use connection::{ConnReceiver, ConnSender};

use crate::core::io::OutgoingFrame;
use crate::protocol::Frame;
use crate::sync::Callback;

use crate::prelude::*;

/// Producing part of channel that sends outgoing frames to a [`Connection`].
///
/// Paired with [`FrameSendHandler`] that usually is owned by a [`Channel`].
pub type FrameSender<V> = mpmc::Sender<OutgoingFrame<V>>;
/// Receiver for incoming frames.
///
/// Paired with [`FrameProducer`] that usually owned by a [`Channel`] and receives incoming frames
/// from the underlying transport.
pub type FrameReceiver<V> = mpmc::Receiver<(Frame<V>, Callback<V>)>;
/// Handles outgoing frames produced by [`Connection::send`].
///
/// Usually owned by channels which intercept outgoing frames and write them to the underlying
/// transport. Paired with [`FrameSender`] which is owned by [`Connection`].
pub type FrameSendHandler<V> = mpmc::Receiver<OutgoingFrame<V>>;
/// Produces incoming frames.
///
/// Owned by a [`Channel`] that reads frames from the underlying transport and emits them to the
/// associated [`Connection`]. Paired with [`FrameReceiver`].
pub type FrameProducer<V> = mpmc::Sender<(Frame<V>, Callback<V>)>;
