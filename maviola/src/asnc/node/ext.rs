//! # 🔒 Asynchronous I/O extensions for node

use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio_stream::Stream;

use crate::asnc::marker::AsyncConnConf;
use crate::core::marker::{Edge, NodeKind, Proxy};
use crate::core::node::{NodeBuilder, NodeConf};
use crate::core::utils::Guarded;
use crate::error::{NodeError, RecvResult, RecvTimeoutResult, TryRecvResult};
use crate::protocol::{Behold, Peer, Unset};

use crate::asnc::prelude::*;
use crate::prelude::*;

impl Node<Proxy, Versionless, Unset> {
    /// <sup>[`async`](crate::asnc)</sup>
    /// Instantiate an empty [`NodeBuilder`] with specified MAVLink protocol version in asynchronous
    /// mode.
    ///
    /// The version either should be specified using [turbofish](https://turbo.fish/about) syntax
    /// or can be derived by Rust compiler.
    pub fn asnc<V: MaybeVersioned>() -> NodeBuilder<Unset, Unset, V, Unset, AsyncApi<V>> {
        NodeBuilder::asynchronous().version::<V>()
    }
}

impl<K: NodeKind, V: MaybeVersioned> Node<K, V, AsyncApi<V>> {
    /// <sup>[`async`](crate::asnc)</sup>
    /// Instantiates node from asynchronous configuration.
    ///
    /// Creates an instance of [`Node`] from [`NodeConf`].
    pub async fn try_from_async_conf(conf: NodeConf<K, V, AsyncConnConf<V>>) -> Result<Self> {
        let (conn, conn_handler) = conf.connection().build().await?;

        let processor = Arc::new(conf.make_processor());
        let api = AsyncApi::new(conn, processor.clone());

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

    /// Returns a reference to an event receiver.
    ///
    /// This receiver can be cloned and passed to other threads.
    ///
    /// **⚠** In order to have access to [`EventReceiver`] methods, you have to import
    /// [`ReceiveEvent`] and [`ReceiveFrame`] traits. You may import [`asnc::prelude`] as well.
    ///
    /// [`asnc::prelude`]: crate::asnc::prelude
    #[inline(always)]
    pub fn receiver(&mut self) -> &mut EventReceiver<V> {
        self.api.event_receiver_mut()
    }

    #[inline(always)]
    pub(in crate::asnc) fn event_receiver(&self) -> &EventReceiver<V> {
        self.api.event_receiver()
    }

    #[inline(always)]
    pub(in crate::asnc) fn frame_sender(&self) -> &FrameSender<V, Proxy> {
        self.api.frame_sender()
    }
}

impl<V: MaybeVersioned> Node<Proxy, V, AsyncApi<V>> {
    /// <sup>[`async`](crate::asnc)</sup>
    /// Returns a new instance of a frame sender.
    ///
    /// Senders can be cloned and passed to other threads.
    ///
    /// Senders returned by [`Proxy`] nodes (i.e. [`ProxyNode`]) can't create frames from MAVLink
    /// messages. This is only possible for [`Edge`] nodes ([`EdgeNode`]) with specified system and
    /// component `ID`s.
    pub fn sender(&self) -> FrameSender<V, Proxy> {
        self.api.frame_sender().clone()
    }
}

impl<V: MaybeVersioned> Node<Edge<V>, V, AsyncApi<V>> {
    /// <sup>[`async`](crate::asnc)</sup>
    /// Returns a new instance of a frame sender that will use the same endpoint settings as the
    /// parent node.
    ///
    /// Senders can be cloned and passed to other threads.
    pub fn sender(&self) -> FrameSender<V, Edge<V>> {
        self.api.frame_sender().clone().into_edge(self.kind.clone())
    }
}

impl<V: Versioned> Node<Edge<V>, V, AsyncApi<V>> {
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

        self.api.start_sending_heartbeats(
            self.kind.endpoint.clone(),
            self.heartbeat_interval,
            self.is_active.clone(),
            self.dialect().version(),
        );

        Ok(())
    }
}

#[async_trait]
impl<K: NodeKind, V: MaybeVersioned> ReceiveEvent<V> for Node<K, V, AsyncApi<V>> {
    #[inline(always)]
    async fn recv(&mut self) -> RecvResult<Event<V>> {
        self.api.event_receiver_mut().recv().await
    }

    #[inline(always)]
    async fn recv_timeout(&mut self, timeout: Duration) -> RecvTimeoutResult<Event<V>> {
        self.api.event_receiver_mut().recv_timeout(timeout).await
    }

    #[inline(always)]
    fn try_recv(&mut self) -> TryRecvResult<Event<V>> {
        self.api.event_receiver_mut().try_recv()
    }

    #[inline(always)]
    fn events(&self) -> Behold<impl Stream<Item = Event<V>>> {
        Behold::new(self.api.events())
    }
}

#[async_trait]
impl<K: NodeKind, V: MaybeVersioned> ReceiveFrame<V> for Node<K, V, AsyncApi<V>> {}
