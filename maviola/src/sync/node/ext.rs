//! # 🔒 Synchronous I/O extensions for node

use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;

use crate::core::marker::{Edge, NodeKind, Proxy};
use crate::core::node::{NodeBuilder, NodeConf};
use crate::core::utils::Guarded;
use crate::error::{NodeError, RecvResult, RecvTimeoutResult, TryRecvResult};
use crate::protocol::{Peer, Unset};
use crate::sync::marker::ConnConf;

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

    #[inline(always)]
    pub(in crate::sync) fn event_receiver(&self) -> &EventReceiver<V> {
        self.api.event_receiver()
    }

    #[inline(always)]
    pub(in crate::sync) fn frame_sender(&self) -> &FrameSender<V, Proxy> {
        self.api.frame_sender()
    }
}

impl<K: NodeKind, V: MaybeVersioned> ReceiveEvent<V> for Node<K, V, SyncApi<V>> {
    #[inline(always)]
    fn recv(&self) -> RecvResult<Event<V>> {
        self.api.event_receiver().recv()
    }

    #[inline(always)]
    fn recv_timeout(&self, timeout: Duration) -> RecvTimeoutResult<Event<V>> {
        self.api.event_receiver().recv_timeout(timeout)
    }

    #[inline(always)]
    fn try_recv(&self) -> TryRecvResult<Event<V>> {
        self.api.event_receiver().try_recv()
    }

    #[inline(always)]
    fn events(&self) -> impl Iterator<Item = Event<V>> {
        self.api.event_receiver().events()
    }
}

impl<K: NodeKind, V: MaybeVersioned> ReceiveFrame<V> for Node<K, V, SyncApi<V>> {}

impl<V: MaybeVersioned> Node<Proxy, V, SyncApi<V>> {
    /// <sup>[`sync`](crate::sync)</sup>
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

impl<V: MaybeVersioned> Node<Edge<V>, V, SyncApi<V>> {
    /// <sup>[`sync`](crate::sync)</sup>
    /// Returns a new instance of a frame sender that will use the same endpoint settings as the
    /// parent node.
    ///
    /// Senders can be cloned and passed to other threads.
    pub fn sender(&self) -> FrameSender<V, Edge<V>> {
        self.api.frame_sender().clone().into_edge(self.kind.clone())
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
