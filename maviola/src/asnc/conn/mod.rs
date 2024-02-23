//! # AsyncConnection and channels
//!
//! This module contains abstractions for connections and channels. [`AsyncConnection`] represents an
//! interface to an underlying transport, while [`AsyncChannel`] is an individual stream withing a
//! connection. Connections are created by implementors of [`AsyncConnectionBuilder`] trait and channels
//! are constructed by [`AsyncChannelFactory`] which is bounded to a particular connection.
//!
//! In most cases channels and connections are hidden to library user. Dealing with these
//! abstractions is necessary only to those who are interested in creating custom connections.

mod channel;
mod connection;

pub use channel::{AsyncChannel, AsyncChannelFactory};
pub use connection::{AsyncConnection, AsyncConnectionBuilder};

pub(crate) use connection::{AsyncConnReceiver, AsyncConnSender};

use crate::asnc::AsyncCallback;
use crate::core::io::OutgoingFrame;
use crate::protocol::Frame;

use crate::prelude::*;

/// Producing part of channel that sends outgoing frames to a [`AsyncConnection`].
///
/// Paired with [`AsyncFrameSendHandler`] that usually is owned by a [`AsyncChannel`].
pub type AsyncFrameSender<V> = broadcast::Sender<OutgoingFrame<V>>;
/// Receiver for incoming frames.
///
/// Paired with [`AsyncFrameProducer`] that usually owned by a [`AsyncChannel`] and receives incoming frames
/// from the underlying transport.
pub type AsyncFrameReceiver<V> = broadcast::Receiver<(Frame<V>, AsyncCallback<V>)>;
/// Handles outgoing frames produced by [`AsyncConnection::send`].
///
/// Usually owned by channels which intercept outgoing frames and write them to the underlying
/// transport. Paired with [`AsyncFrameSender`] which is owned by [`AsyncConnection`].
pub type AsyncFrameSendHandler<V> = broadcast::Receiver<OutgoingFrame<V>>;
/// Produces incoming frames.
///
/// Owned by a [`AsyncChannel`] that reads frames from the underlying transport and emits them to the
/// associated [`AsyncConnection`]. Paired with [`AsyncFrameReceiver`].
pub type AsyncFrameProducer<V> = broadcast::Sender<(Frame<V>, AsyncCallback<V>)>;
