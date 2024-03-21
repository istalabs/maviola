//! # 🔒 Synchronous I/O extensions for node

use mavio::protocol::Unset;
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;

use crate::core::marker::{Edge, NodeKind, Proxy};
use crate::core::node::{NodeBuilder, NodeConf};
use crate::core::utils::Guarded;
use crate::error::NodeError;
use crate::protocol::Peer;
use crate::sync::marker::ConnConf;
use crate::sync::node::api::{EventReceiver, FrameSender};

use crate::prelude::*;
use crate::sync::prelude::*;

impl Node<Proxy, Versionless, Unset> {
    /// <sup>[`sync`](crate::sync)</sup>
    /// Instantiate an empty [`NodeBuilder`] with specified MAVLink protocol version in asynchronous
    /// mode.
    ///
    /// The version either should be specified using [turbofish](https://turbo.fish/about) syntax
    /// or can be derived by Rust compiler.
    pub fn sync<V: MaybeVersioned>() -> NodeBuilder<Unset, Unset, V, Unset, SyncApi<V>> {
        NodeBuilder::synchronous().version::<V>()
    }
}

impl<K: NodeKind, V: MaybeVersioned> Node<K, V, SyncApi<V>> {
    /// <sup>[`sync`](crate::sync)</sup>
    /// Instantiates node from synchronous configuration.
    ///
    /// Creates an instance of [`Node`] from [`NodeConf`].
    pub fn try_from_conf(conf: NodeConf<K, V, ConnConf<V>>) -> Result<Self> {
        let (conn, conn_handler) = conf.connection().build()?;

        let processor = Arc::new(conf.make_processor());
        let api = SyncApi::new(conn, processor.clone());

        let state = api.share_state();
        let is_active = Guarded::from(&state);

        let node = Self {
            kind: conf.kind,
            api,
            state,
            is_active,
            heartbeat_timeout: conf.heartbeat_timeout,
            heartbeat_interval: conf.heartbeat_interval,
            processor,
            _version: PhantomData,
        };

        node.api.start_default_handlers(node.heartbeat_timeout);
        node.api.handle_conn_stop(conn_handler);

        Ok(node)
    }

    /// <sup>[`sync`](crate::sync)</sup>
    /// Returns `true` if node has connected MAVLink peers.
    ///
    /// Disconnected node will always return `false`.
    pub fn has_peers(&self) -> bool {
        self.api.has_peers()
    }

    /// <sup>[`sync`](crate::sync)</sup>
    /// Returns an iterator over current peers.
    ///
    /// This method will return a snapshot of the current peers relevant to the time when it was
    /// called. A more reliable approach to peer management is to use [`Node::events`] and track
    /// [`Event::NewPeer`] / [`Event::PeerLost`] events.
    pub fn peers(&self) -> impl Iterator<Item = Peer> {
        self.api.peers()
    }

    /// <sup>[`sync`](crate::sync)</sup>
    /// Receives the next node [`Event`].
    ///
    /// Blocks until event received.
    pub fn recv(&self) -> Result<Event<V>> {
        self.api.recv_event()
    }

    /// <sup>[`sync`](crate::sync)</sup>
    /// Attempts to receive the next node [`Event`] within a `timeout`.
    ///
    /// Blocks until event received or deadline is reached.
    pub fn recv_timeout(&self, timeout: Duration) -> Result<Event<V>> {
        self.api.recv_event_timeout(timeout)
    }

    /// <sup>[`sync`](crate::sync)</sup>
    /// Attempts to receive MAVLink [`Event`] without blocking.
    pub fn try_recv(&self) -> Result<Event<V>> {
        self.api.try_recv_event()
    }

    /// <sup>[`sync`](crate::sync)</sup>
    /// Subscribe to node events.
    ///
    /// Blocks while the node is active.
    ///
    /// If you are interested only in valid incoming frames, use [`Node::recv_frame`] or
    /// [`Node::try_recv_frame`] instead.
    pub fn events(&self) -> impl Iterator<Item = Event<V>> {
        self.api.events()
    }

    /// <sup>[`sync`](crate::sync)</sup>
    /// Receives the next frame. Blocks until valid frame received or channel is closed.
    ///
    /// If you want to block until the next frame within a timeout, use [`Node::recv_frame_timeout`].
    /// If you want to check for the next frame without blocking, use [`Node::try_recv_frame`].
    ///
    /// **⚠** This method skips all invalid frames. If you are interested in such frames, use
    /// [Node::events] or [`Node::recv`] instead to receive [`crate::asnc::node::Event::Invalid`] events that
    /// contain invalid frame with the corresponding error.
    pub fn recv_frame(&self) -> Result<(Frame<V>, Callback<V>)> {
        self.api.recv_frame()
    }

    /// <sup>[`sync`](crate::sync)</sup>
    /// Attempts ot receives the next frame until the timeout is reached. Blocks until valid frame
    /// received, deadline is reached, or channel is closed.
    ///
    /// If you want to block until the next frame is received, use [`Node::recv_frame`].
    /// If you want to check for the next frame without blocking, use [`Node::try_recv_frame`].
    ///
    /// **⚠** This method skips all invalid frames. If you are interested in such frames, use
    /// [Node::events] or [`Node::recv`] instead to receive [`Event::Invalid`] events that
    /// contain invalid frame with the corresponding error.
    pub fn recv_frame_timeout(&self, timeout: Duration) -> Result<(Frame<V>, Callback<V>)> {
        self.api.recv_frame_timeout(timeout)
    }

    /// <sup>[`sync`](crate::sync)</sup>
    /// Attempts to receive the next valid frame. Returns immediately if channel is empty.
    ///
    /// If you want to block until the next frame within a timeout, use [`Node::recv_frame_timeout`].
    /// If you want to block until the next frame is received, use [`Node::recv_frame`].
    ///
    /// **⚠** This method skips all invalid frames. If you are interested in such frames, use
    /// [Node::events] or [`Node::try_recv`] instead to receive [`Event::Invalid`] events that
    /// contain invalid frame with the corresponding error.
    pub fn try_recv_frame(&self) -> Result<(Frame<V>, Callback<V>)> {
        self.api.try_recv_frame()
    }

    pub(in crate::sync) fn event_receiver(&self) -> &EventReceiver<V> {
        self.api.event_receiver()
    }

    pub(in crate::sync) fn frame_sender(&self) -> &FrameSender<V> {
        self.api.frame_sender()
    }
}

impl<V: Versioned> Node<Edge<V>, V, SyncApi<V>> {
    /// <sup>[`sync`](crate::sync)</sup>
    /// Activates the node.
    ///
    /// Active nodes emit heartbeats and perform other operations which do not depend on user
    /// initiative directly.
    ///
    /// This method is available only for nodes which are [`Edge`] and [`Versioned`].
    ///
    /// [`Node::activate`] is idempotent while node is connected. Otherwise, it will return
    /// [`NodeError::Inactive`] variant of [`Error::Node`].
    pub fn activate(&mut self) -> Result<()> {
        if self.state.is_closed() {
            return Err(Error::Node(NodeError::Inactive));
        }

        if self.is_active.is() {
            return Ok(());
        }

        self.is_active.set(true);

        self.api.start_sending_heartbeats(
            self.kind.endpoint.clone(),
            self.heartbeat_interval,
            self.is_active.clone(),
            self.dialect().version(),
        );

        Ok(())
    }
}
