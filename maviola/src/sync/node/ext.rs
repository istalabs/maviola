//! # 🔒 Synchronous I/O extensions for node

use std::marker::PhantomData;
use std::sync::atomic::AtomicU8;
use std::sync::Arc;

use crate::core::marker::{Identified, MaybeIdentified};
use crate::core::utils::Guarded;
use crate::core::{Node, NodeConf};
use crate::protocol::Peer;
use crate::sync::marker::ConnConf;
use crate::sync::node::api::SyncApi;
use crate::sync::{Callback, Event};

use crate::prelude::*;

impl<I: MaybeIdentified, D: Dialect, V: MaybeVersioned + 'static> Node<I, D, V, SyncApi<V>> {
    /// <sup>[`sync`](crate::sync)</sup>
    /// Instantiates node from node configuration.
    ///
    /// Creates ona instance of [`Node`] from [`NodeConf`]. It is also possible to use [`TryFrom`]
    /// and create a node with [`Node::try_from`].
    pub fn try_from_conf(conf: NodeConf<I, D, V, ConnConf<V>>) -> Result<Self> {
        let api = SyncApi::new(conf.connection().build()?);
        let state = api.share_state();
        let is_active = Guarded::from(&state);

        let node = Self {
            id: conf.id,
            version: conf.version,
            api,
            state,
            is_active,
            sequence: Arc::new(AtomicU8::new(0)),
            heartbeat_timeout: conf.heartbeat_timeout,
            heartbeat_interval: conf.heartbeat_interval,
            _dialect: PhantomData,
        };

        node.api.start_default_handlers(node.heartbeat_timeout);

        Ok(node)
    }

    /// <sup>[`sync`](crate::sync)</sup>
    /// Receive MAVLink message blocking until MAVLink frame received.
    pub fn recv(&self) -> Result<(D, Callback<V>)> {
        let (frame, res) = self.recv_frame_internal()?;
        let msg = D::decode(frame.payload())?;
        Ok((msg, res))
    }

    /// <sup>[`sync`](crate::sync)</sup>
    /// Attempts to receive MAVLink message without blocking.
    pub fn try_recv(&self) -> Result<(D, Callback<V>)> {
        let (frame, res) = self.try_recv_frame_internal()?;
        let msg = D::decode(frame.payload())?;
        Ok((msg, res))
    }

    /// Request the next node [`Event`].
    ///
    /// Blocks until event received.
    pub fn recv_event(&self) -> Result<Event<V>> {
        self.api.recv_event()
    }

    /// Attempts to receive MAVLink [`Event`] without blocking.
    pub fn try_recv_event(&self) -> Result<Event<V>> {
        self.api.try_recv_event()
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
    /// Proxy MAVLink [`Frame`].
    ///
    /// In proxy mode [`Frame`] is sent with as many fields preserved as possible. However, the
    /// following properties could be updated based on the node's
    /// [message signing](https://mavlink.io/en/guide/message_signing.html) configuration
    /// (`MAVLink 2` [`Versioned`] nodes only):
    ///
    /// * [`signature`](Frame::signature)
    /// * [`link_id`](Frame::link_id)
    /// * [`timestamp`](Frame::timestamp)
    ///
    /// To send MAVLink messages instead of raw frames, construct an [`Identified`] node and use
    /// messages [`Node::send_versioned`] for node which is [`Versionless`] and [`Node::send`] for
    /// [`Versioned`] nodes. In the latter case, message will be encoded according to MAVLink
    /// protocol version defined for a node.
    pub fn proxy_frame(&self, frame: &Frame<V>) -> Result<()> {
        self.send_frame_internal(frame)
    }

    /// <sup>[`sync`](crate::sync)</sup>
    /// Receive MAVLink [`Frame`].
    ///
    /// Blocks until frame received.
    pub fn recv_frame(&self) -> Result<(Frame<V>, Callback<V>)> {
        self.recv_frame_internal()
    }

    /// <sup>[`sync`](crate::sync)</sup>
    /// Attempts to receive MAVLink [`Frame`] without blocking.
    pub fn try_recv_frame(&self) -> Result<(Frame<V>, Callback<V>)> {
        self.try_recv_frame_internal()
    }

    /// <sup>[`sync`](crate::sync)</sup>
    /// Subscribe to node events.
    ///
    /// Blocks while the node is active.
    pub fn events(&self) -> impl Iterator<Item = Event<V>> {
        self.api.events()
    }

    fn recv_frame_internal(&self) -> Result<(Frame<V>, Callback<V>)> {
        self.api.recv_frame()
    }

    fn try_recv_frame_internal(&self) -> Result<(Frame<V>, Callback<V>)> {
        self.api.try_recv_frame()
    }

    fn send_frame_internal(&self, frame: &Frame<V>) -> Result<()> {
        self.api.send_frame(frame)
    }
}

impl<D: Dialect, V: Versioned + 'static> Node<Identified, D, V, SyncApi<V>> {
    /// <sup>[`sync`](crate::sync)</sup>
    /// Activates the node.
    ///
    /// Active nodes emit heartbeats and perform other operations which do not depend on user
    /// initiative directly.
    ///
    /// This method is available only for nodes which are [`Identified`].
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

        self.api.start_sending_heartbeats::<D>(
            self.id.clone(),
            self.heartbeat_interval,
            self.sequence.clone(),
            self.is_active.clone(),
        );

        Ok(())
    }
}
