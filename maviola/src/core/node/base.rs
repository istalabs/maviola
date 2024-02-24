use std::marker::PhantomData;
use std::sync::atomic::AtomicU8;
use std::sync::{atomic, Arc};
use std::time::Duration;

use crate::core::io::ConnectionInfo;
use crate::core::marker::{
    Identified, MaybeIdentified, NoComponentId, NoConnConf, NoSystemId, Unidentified,
};
use crate::core::node::api::{NoApi, NodeApi};
use crate::core::utils::{Guarded, SharedCloser, Switch};
use crate::core::NodeBuilder;
use crate::protocol::{
    ComponentId, Dialect, Frame, MavLinkVersion, MaybeVersioned, Message, SystemId, Versioned,
    Versionless,
};

use crate::prelude::*;

/// MAVLink node.
///
/// # Examples
///
/// Create a TCP server node:
///
/// ```rust
/// # #[cfg(feature = "sync")]
/// # {
/// use maviola::protocol::{Peer, V2};
/// use maviola::sync::{Event, TcpServer};
/// use maviola::core::Node;
/// use maviola::dialects::Minimal;
/// # use portpicker::pick_unused_port;
///
/// let addr = "127.0.0.1:5600";
/// # let addr = format!("127.0.0.1:{}", pick_unused_port().unwrap());
///
/// // Create a node from configuration.
/// let mut node = Node::try_from(
///     Node::builder()
///         .version(V2)                // restrict node to MAVLink2 protocol version
///         .system_id(1)               // System `ID`
///         .component_id(1)            // Component `ID`
///         .dialect::<Minimal>()       // Dialect is set to `minimal`
///         .connection(
///             TcpServer::new(addr)    // Configure TCP server connection
///                 .unwrap()
///         )
/// ).unwrap();
///
/// // Activate node to start sending heartbeats.
/// node.activate().unwrap();
/// # struct __Struct(); impl __Struct { fn events(&self) -> Vec<Event<V2>> { vec![Event::NewPeer(Peer::new(0, 0))] } }
/// # let node = __Struct();
///
/// for event in node.events() {
///     match event {
///         Event::NewPeer(peer) => {
///             /* handle a new peer */
/// #           drop(peer);
///         }
///         Event::PeerLost(peer) => {
///             /* handle a peer, that becomes inactive */
/// #           drop(peer);
///         }
///         Event::Frame(frame, res) => {
///             // Send back any incoming frame directly to sender.
///             res.respond(&frame).unwrap();
///         }
///     }
/// }
/// # }
/// ```
pub struct Node<I: MaybeIdentified, D: Dialect, V: MaybeVersioned + 'static, A: NodeApi<V>> {
    pub(crate) id: I,
    pub(crate) version: V,
    pub(crate) api: A,
    pub(crate) sequence: Arc<AtomicU8>,
    pub(crate) state: SharedCloser,
    pub(crate) is_active: Guarded<SharedCloser, Switch>,
    pub(crate) heartbeat_timeout: Duration,
    pub(crate) heartbeat_interval: Duration,
    pub(crate) _dialect: PhantomData<D>,
}

impl Node<Unidentified, Minimal, Versionless, NoApi> {
    /// Instantiates an empty [`NodeBuilder`].
    pub fn builder() -> NodeBuilder<NoSystemId, NoComponentId, Minimal, Versionless, NoConnConf> {
        NodeBuilder::new()
    }
}

impl<I: MaybeIdentified, D: Dialect, V: MaybeVersioned + 'static, A: NodeApi<V>> Node<I, D, V, A> {
    /// Information about this node's connection.
    pub fn info(&self) -> &ConnectionInfo {
        self.api.info()
    }

    /// Returns `true` if node has connected MAVLink peers.
    ///
    /// Disconnected node will always return `false`.
    pub fn has_peers(&self) -> bool {
        self.api.has_peers()
    }

    /// Heartbeat timeout.
    ///
    /// For peers that overdue to send the next heartbeat within this interval will be considered
    /// inactive.
    ///
    /// Default value is [`DEFAULT_HEARTBEAT_TIMEOUT`](crate::core::consts::DEFAULT_HEARTBEAT_TIMEOUT).
    pub fn heartbeat_timeout(&self) -> Duration {
        self.heartbeat_timeout
    }

    /// Returns `true` if node is connected.
    ///
    /// All nodes are connected by default, they can become disconnected only if I/O transport
    /// failed or have been exhausted.
    pub fn is_connected(&self) -> bool {
        !self.state.is_closed()
    }
}

impl<D: Dialect, V: Versioned + 'static, A: NodeApi<V>> Node<Identified, D, V, A> {
    /// <sup>[`sync`](crate::sync)</sup>
    /// Send MAVLink message.
    ///
    /// The message will be encoded according to the node's dialect specification and MAVLink
    /// protocol version.
    ///
    /// If you want to send messages within different MAVLink protocols simultaneously, you have
    /// to construct a [`Versionless`] node and use [`Node::send_versioned`]
    pub fn send(&self, message: &impl Message) -> Result<()> {
        let frame = self.make_frame_from_message(message, self.version.clone())?;
        self.api.send_frame(&frame)
    }

    /// Returns `true`, if node is active.
    ///
    /// All nodes are inactive by default and have to be activated using [`Node::activate`].
    ///
    /// Active nodes will send heartbeats and perform other automated operations which do not
    /// require direct initiative from the user.
    ///
    /// Inactive nodes will neither send heartbeats, nor perform other operations which are not
    /// directly requested by user. They will still receive incoming frames and emit corresponding
    /// events.
    ///
    /// Active nodes are also connected and [`Node::is_connected`] will return `true`.
    ///
    /// Node transitions into inactive state when it becomes disconnected or when
    /// [`Node::deactivate`] called.
    pub fn is_active(&self) -> bool {
        self.is_active.is()
    }

    /// Heartbeat interval.
    ///
    /// Once node is started using [`Node::activate`], it will emit heartbeats with this interval.
    ///
    /// Default value is [`DEFAULT_HEARTBEAT_INTERVAL`](crate::core::consts::DEFAULT_HEARTBEAT_INTERVAL).
    pub fn heartbeat_interval(&self) -> Duration {
        self.heartbeat_interval
    }

    /// Deactivates the node.
    ///
    /// Inactive nodes will neither send heartbeats, nor perform other operations which are not
    /// directly requested by user. They will still receive incoming frames and emit corresponding
    /// events.
    ///
    /// [`Node::deactivate`] is idempotent while node is connected. Otherwise, it will return
    /// [`NodeError::Inactive`] variant of [`Error::Node`].
    pub fn deactivate(&mut self) -> Result<()> {
        if self.state.is_closed() {
            return Err(Error::Node(NodeError::Inactive));
        }

        if !self.is_active.is() {
            return Ok(());
        }

        self.is_active.set(false);

        Ok(())
    }
}

impl<D: Dialect, V: MaybeVersioned, A: NodeApi<V>> Node<Identified, D, V, A> {
    /// MAVLink system ID.
    pub fn system_id(&self) -> SystemId {
        self.id.system_id
    }

    /// MAVLink component ID.
    pub fn component_id(&self) -> ComponentId {
        self.id.component_id
    }

    pub(crate) fn make_frame_from_message<Version: Versioned>(
        &self,
        message: &impl Message,
        version: Version,
    ) -> Result<Frame<Version>> {
        let sequence = self.sequence.fetch_add(1, atomic::Ordering::Relaxed);
        let payload = message.encode(Version::version())?;
        let frame = Frame::builder()
            .sequence(sequence)
            .system_id(self.id.system_id)
            .component_id(self.id.component_id)
            .payload(payload)
            .crc_extra(message.crc_extra())
            .version(version)
            .build();
        Ok(frame)
    }
}

impl<I: MaybeIdentified, D: Dialect, V: Versioned, A: NodeApi<V>> Node<I, D, V, A> {
    /// MAVLink version.
    pub fn version(&self) -> MavLinkVersion {
        V::version()
    }
}

impl<D: Dialect, A: NodeApi<Versionless>> Node<Identified, D, Versionless, A> {
    /// Send MAVLink frame with a specified MAVLink protocol version.
    ///
    /// If you want to restrict MAVLink protocol to a particular version, construct a [`Versioned`]
    /// node and simply send messages by calling [`Node::send`].
    pub fn send_versioned<V: Versioned>(&self, message: &impl Message, version: V) -> Result<()> {
        let frame = self
            .make_frame_from_message(message, version)?
            .versionless();
        self.api.send_frame(&frame)
    }
}

impl<I: MaybeIdentified, D: Dialect, V: MaybeVersioned + 'static, A: NodeApi<V>> Drop
    for Node<I, D, V, A>
{
    fn drop(&mut self) {
        self.state.close();

        log::debug!("{:?}: node is dropped", self.info());
    }
}
