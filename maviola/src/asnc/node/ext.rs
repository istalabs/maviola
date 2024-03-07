//! # 🔒 Asynchronous I/O extensions for node

use std::marker::PhantomData;
use std::sync::Arc;
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
    /// Instantiates node from asynchronous configuration.
    ///
    /// Creates an instance of [`Node`] from [`NodeConf`].
    pub async fn try_from_async_conf(conf: NodeConf<K, D, V, AsyncConnConf<V>>) -> Result<Self> {
        let (conn, conn_handler) = conf.connection().build().await?;

        let signer = conf.signer.clone().map(Arc::new);
        let api = AsyncApi::new(conn, signer.clone());

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
    /// Returns a stream over current peers.
    ///
    /// This method will return a snapshot of the current peers relevant to the time when it was
    /// called. A more reliable approach to peer management is to use [`Node::events`] and track
    /// [`Event::NewPeer`] / [`Event::PeerLost`] events.
    pub async fn peers(&self) -> impl Stream<Item = Peer> {
        self.api.peers().await
    }

    /// <sup>[`async`](crate::asnc)</sup>
    /// Request the next node [`Event`].
    ///
    /// Blocks until event received.
    ///
    /// If you are interested only in valid incoming frames, use [`Node::recv_frame`] instead.
    pub async fn recv(&mut self) -> Result<Event<V>> {
        self.api.recv_event().await
    }

    /// <sup>[`async`](crate::asnc)</sup>
    /// Attempts to receive MAVLink [`Event`] without blocking.
    ///
    /// If you are interested only in valid incoming frames, use [`Node::try_recv_frame`] instead.
    pub fn try_recv(&mut self) -> Result<Event<V>> {
        self.api.try_recv_event()
    }

    /// <sup>[`async`](crate::asnc)</sup>
    /// Subscribe to node events.
    ///
    /// Returns a stream of node events. Requires [`StreamExt`] from Tokio stream extensions to be
    /// imported (you may use [`asnc::prelude`](crate::asnc::prelude) that imports it as well).
    ///
    /// If you are interested only in valid incoming frames, use [`Node::recv_frame`] or
    /// [`Node::try_recv_frame`] instead.
    ///
    /// ⚠ The result is wrapped with [`Behold`] as a reminder, that the returned stream will have
    /// access only to events that were emitted close to the moment, when the method was called.
    /// Repetitive calls in the loop may lead to undesired behavior. This is related to the nature
    /// of the asynchronous MPMC channels, that able to operate only on a limited number of the past
    /// events.
    pub fn events(&self) -> Behold<impl Stream<Item = Event<V>>> {
        Behold::new(self.api.events())
    }

    /// <sup>[`async`](crate::asnc)</sup>
    /// Receives the next frame. Blocks until valid frame received or channel is closed.
    ///
    /// If you want to check for the next frame without blocking, use [`Node::try_recv_frame`].
    ///
    /// **⚠** This method skips all invalid frames. If you are interested in such frames, use
    /// [Node::events] or [`Node::recv`] instead to receive [`Event::Invalid`] events that
    /// contain invalid frame with the corresponding error.
    pub async fn recv_frame(&mut self) -> Result<(Frame<V>, Callback<V>)> {
        self.api.recv_frame().await
    }

    /// <sup>[`async`](crate::asnc)</sup>
    /// Attempts to receive the next valid frame.
    ///
    /// This method returns immediately if channel is empty. If you want to block until the next
    /// frame is received, use [`Node::recv_frame`].
    ///
    /// **⚠** This method skips all invalid frames. If you are interested in such frames, use
    /// [Node::events] or [`Node::try_recv`] instead to receive [`Event::Invalid`] events that
    /// contain invalid frame with the corresponding error.
    pub fn try_recv_frame(&mut self) -> Result<(Frame<V>, Callback<V>)> {
        self.api.try_recv_frame()
    }
}

impl<D: Dialect, V: Versioned + 'static> Node<Edge<V>, D, V, AsyncApi<V>> {
    /// <sup>[`async`](crate::asnc)</sup>
    /// Activates the node.
    ///
    /// Active nodes emit heartbeats and perform other operations which do not depend on user
    /// initiative directly.
    ///
    /// This method is available only for nodes which are [`Edge`] and [`Versioned`].
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

        self.api.start_sending_heartbeats::<D>(
            self.kind.endpoint.clone(),
            self.heartbeat_interval,
            self.is_active.clone(),
        );

        Ok(())
    }
}
