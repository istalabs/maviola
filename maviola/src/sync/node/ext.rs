//! # 🔒 Synchronous I/O extensions for node

use std::marker::PhantomData;

use crate::core::marker::{Edge, NodeKind};
use crate::core::node::NodeConf;
use crate::core::utils::Guarded;
use crate::protocol::Peer;
use crate::sync::marker::ConnConf;

use crate::prelude::*;
use crate::sync::prelude::*;

impl<K: NodeKind, D: Dialect, V: MaybeVersioned + 'static> Node<K, D, V, SyncApi<V>> {
    /// <sup>[`sync`](crate::sync)</sup>
    /// Instantiates node from synchronous node configuration.
    ///
    /// Creates ona instance of [`Node`] from [`NodeConf`].
    pub fn try_from_conf(conf: NodeConf<K, D, V, ConnConf<V>>) -> Result<Self> {
        let (conn, conn_handler) = conf.connection().build()?;
        let api = SyncApi::new(conn);
        let state = api.share_state();
        let is_active = Guarded::from(&state);

        let node = Self {
            kind: conf.kind,
            api,
            state,
            is_active,
            heartbeat_timeout: conf.heartbeat_timeout,
            heartbeat_interval: conf.heartbeat_interval,
            _dialect: PhantomData,
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
    /// Receive MAVLink message blocking until MAVLink frame received.
    pub fn recv(&self) -> Result<(D, Callback<V>)> {
        let (frame, res) = self.api.recv_frame()?;
        let msg = D::decode(frame.payload())?;
        Ok((msg, res))
    }

    /// <sup>[`sync`](crate::sync)</sup>
    /// Attempts to receive MAVLink message without blocking.
    pub fn try_recv(&self) -> Result<(D, Callback<V>)> {
        let (frame, res) = self.api.try_recv_frame()?;
        let msg = D::decode(frame.payload())?;
        Ok((msg, res))
    }

    /// <sup>[`sync`](crate::sync)</sup>
    /// Request the next node [`Event`].
    ///
    /// Blocks until event received.
    pub fn recv_event(&self) -> Result<Event<V>> {
        self.api.recv_event()
    }

    /// <sup>[`sync`](crate::sync)</sup>
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
    /// To send MAVLink messages instead of raw frames, construct an [`Edge`] node and use
    /// [`Node::send_versioned`] for node which is [`Versionless`] and [`Node::send`] for
    /// [`Versioned`] nodes. In the latter case, message will be encoded according to MAVLink
    /// protocol version defined for a node.
    pub fn proxy_frame(&self, frame: &Frame<V>) -> Result<()> {
        self.api.send_frame(frame)
    }

    /// <sup>[`sync`](crate::sync)</sup>
    /// Receive MAVLink [`Frame`].
    ///
    /// Blocks until frame received.
    pub fn recv_frame(&self) -> Result<(Frame<V>, Callback<V>)> {
        self.api.recv_frame()
    }

    /// <sup>[`sync`](crate::sync)</sup>
    /// Attempts to receive MAVLink [`Frame`] without blocking.
    pub fn try_recv_frame(&self) -> Result<(Frame<V>, Callback<V>)> {
        self.api.try_recv_frame()
    }

    /// <sup>[`sync`](crate::sync)</sup>
    /// Subscribe to node events.
    ///
    /// Blocks while the node is active.
    pub fn events(&self) -> impl Iterator<Item = Event<V>> {
        self.api.events()
    }
}

impl<D: Dialect, V: Versioned + 'static> Node<Edge<V>, D, V, SyncApi<V>> {
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

        self.api.start_sending_heartbeats::<D>(
            self.kind.endpoint.clone(),
            self.heartbeat_interval,
            self.is_active.clone(),
        );

        Ok(())
    }
}
