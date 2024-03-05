//! # 🔒 Asynchronous I/O extensions for node

use std::marker::PhantomData;
use tokio_stream::Stream;

use crate::asnc::marker::AsyncConnConf;
use crate::core::marker::{Edge, NodeKind};
use crate::core::node::NodeConf;
use crate::core::utils::Guarded;
use crate::protocol::{Behold, Peer};

use crate::asnc::prelude::*;
use crate::prelude::*;

impl<K: NodeKind, D: Dialect, V: MaybeVersioned + 'static> Node<K, D, V, AsyncApi<V>> {
    /// <sup>[`async`](crate::asnc)</sup>
    /// Instantiates node from asynchronous node configuration.
    ///
    /// Creates ona instance of [`Node`] from [`NodeConf`].
    pub async fn try_from_async_conf(conf: NodeConf<K, D, V, AsyncConnConf<V>>) -> Result<Self> {
        let (conn, conn_handler) = conf.connection().build().await?;
        let api = AsyncApi::new(conn);
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

        node.api
            .start_default_handlers(node.heartbeat_timeout)
            .await;
        node.api.handle_conn_stop(conn_handler).await;

        Ok(node)
    }

    /// <sup>[`async`](crate::asnc)</sup>
    /// Returns `true` if node has connected MAVLink peers.
    ///
    /// Disconnected node will always return `false`.
    pub async fn has_peers(&self) -> bool {
        self.api.has_peers().await
    }

    /// <sup>[`async`](crate::asnc)</sup>
    /// Receive MAVLink message blocking until MAVLink frame received.
    pub async fn recv(&mut self) -> Result<(D, Callback<V>)> {
        let (frame, res) = self.recv_frame_internal().await?;
        let msg = D::decode(frame.payload())?;
        Ok((msg, res))
    }

    /// <sup>[`async`](crate::asnc)</sup>
    /// Attempts to receive MAVLink message without blocking.
    pub fn try_recv(&mut self) -> Result<(D, Callback<V>)> {
        let (frame, res) = self.try_recv_frame_internal()?;
        let msg = D::decode(frame.payload())?;
        Ok((msg, res))
    }

    /// <sup>[`async`](crate::asnc)</sup>
    /// Request the next node [`Event`].
    ///
    /// Blocks until event received.
    pub async fn recv_event(&mut self) -> Result<Event<V>> {
        self.api.recv_event().await
    }

    /// <sup>[`async`](crate::asnc)</sup>
    /// Attempts to receive MAVLink [`Event`] without blocking.
    pub fn try_recv_event(&mut self) -> Result<Event<V>> {
        self.api.try_recv_event()
    }

    /// <sup>[`async`](crate::asnc)</sup>
    /// Returns a stream over current peers.
    ///
    /// This method will return a snapshot of the current peers relevant to the time when it was
    /// called. A more reliable approach to peer management is to use [`Node::events`] and track
    /// [`Event::NewPeer`] / [`Event::PeerLost`] events.
    pub async fn peers(&self) -> impl Stream<Item = Peer> {
        self.api.peers().await
    }

    /// <sup>[`async`](crate::asnc)</sup>
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
    /// messages [`Node::send_versioned`] for node which is [`Versionless`] and [`Node::send`] for
    /// [`Versioned`] nodes. In the latter case, message will be encoded according to MAVLink
    /// protocol version defined for a node.
    pub fn proxy_frame(&self, frame: &Frame<V>) -> Result<()> {
        self.send_frame_internal(frame)
    }

    /// <sup>[`async`](crate::asnc)</sup>
    /// Receive MAVLink [`Frame`].
    ///
    /// Blocks until frame received.
    pub async fn recv_frame(&mut self) -> Result<(Frame<V>, Callback<V>)> {
        self.recv_frame_internal().await
    }

    /// <sup>[`async`](crate::asnc)</sup>
    /// Attempts to receive MAVLink [`Frame`] without blocking.
    pub fn try_recv_frame(&mut self) -> Result<(Frame<V>, Callback<V>)> {
        self.try_recv_frame_internal()
    }

    /// <sup>[`async`](crate::asnc)</sup>
    /// Subscribe to node events.
    ///
    /// Returns a stream of node events.
    ///
    /// ⚠ The result is wrapped with [`Behold`] as a reminder that the returned stream will have
    /// access only to events that were emitted close to the moment when the method is called and
    /// repetitive calls may lead to undesired behavior. This is related to the nature of the
    /// asynchronous MPMC channels, that able to operate only on a limited number of the past
    /// events.
    pub fn events(&self) -> Behold<impl Stream<Item = Event<V>>> {
        Behold::new(self.api.events())
    }

    async fn recv_frame_internal(&mut self) -> Result<(Frame<V>, Callback<V>)> {
        self.api.recv_frame().await
    }

    fn try_recv_frame_internal(&mut self) -> Result<(Frame<V>, Callback<V>)> {
        self.api.try_recv_frame()
    }

    fn send_frame_internal(&self, frame: &Frame<V>) -> Result<()> {
        self.api.send_frame(frame)
    }
}

impl<D: Dialect, V: Versioned + 'static> Node<Edge<V>, D, V, AsyncApi<V>> {
    /// <sup>[`async`](crate::asnc)</sup>
    /// Activates the node.
    ///
    /// Active nodes emit heartbeats and perform other operations which do not depend on user
    /// initiative directly.
    ///
    /// This method is available only for nodes which are [`Edge`].
    ///
    /// [`Node::activate`] is idempotent while node is connected. Otherwise, it will return
    /// [`NodeError::Inactive`] variant of [`Error::Node`].
    pub async fn activate(&mut self) -> Result<()> {
        if self.state.is_closed() {
            return Err(Error::Node(NodeError::Inactive));
        }

        if self.is_active.is() {
            return Ok(());
        }

        self.is_active.set(true);

        self.api
            .start_sending_heartbeats::<D>(
                self.kind.endpoint.clone(),
                self.heartbeat_interval,
                self.is_active.clone(),
            )
            .await;

        Ok(())
    }
}
