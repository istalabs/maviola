//! MAVLink node configuration.

use std::marker::PhantomData;
use std::time::Duration;

use crate::core::consts::DEFAULT_HEARTBEAT_INTERVAL;
use crate::core::marker::{Edge, HasConnConf, MaybeConnConf, NodeKind, Proxy, Unset};
use crate::core::node::NodeBuilder;
use crate::protocol::{
    ComponentId, CustomFrameProcessors, DialectSpec, FrameProcessor, FrameSigner, KnownDialects,
    SystemId,
};

use crate::prelude::*;

/// MAVLink node configuration.
///
/// Node configuration can be instantiated only through [`NodeBuilder`]. Once node configuration is
/// obtained from [`NodeBuilder::conf`], it can be used to construct a node by [`NodeConf::build`]
/// or updated via [`NodeConf::update`]. The latter will turn node configuration to a
/// [`NodeBuilder`] populated with current settings.
#[derive(Clone, Debug)]
pub struct NodeConf<K: NodeKind, V: MaybeVersioned, C: MaybeConnConf> {
    pub(crate) kind: K,
    pub(crate) connection_conf: C,
    pub(crate) heartbeat_timeout: Duration,
    pub(crate) heartbeat_interval: Duration,
    pub(crate) dialects: KnownDialects,
    pub(crate) signer: Option<FrameSigner>,
    pub(crate) compat: Option<CompatProcessor>,
    pub(crate) processors: CustomFrameProcessors,
    pub(crate) _version: PhantomData<V>,
}

/// Implementors of this trait can be converted into node configuration.
///
/// Currently, this trait is implemented for [`NodeConf`] itself and [`NodeBuilder`].
pub trait IntoNodeConf<K: NodeKind, V: MaybeVersioned, C: MaybeConnConf> {
    /// Converts into a [`NodeConf`].
    fn into_node_conf(self) -> NodeConf<K, V, C>;
}

impl NodeConf<Proxy, Versionless, Unset> {
    /// Creates an empty [`NodeBuilder`].
    ///
    /// # Usage
    ///
    /// Create a synchronous node configuration.
    ///
    /// ```rust
    /// use maviola::core::node::NodeConf;
    /// use maviola::core::io::TcpClient;
    ///
    /// let node = NodeConf::builder()
    ///     .sync()
    ///     .system_id(10)
    ///     .component_id(42)
    ///     .connection(TcpClient::new("localhost:5600").unwrap())
    ///     .conf();
    ///
    /// assert_eq!(node.system_id(), 10);
    /// assert_eq!(node.component_id(), 42);
    /// ```
    ///
    /// Create a configuration for unidentified node.
    ///
    /// ```rust
    /// use maviola::core::node::NodeConf;
    /// use maviola::core::io::TcpClient;
    ///
    /// let node = NodeConf::builder()
    ///     .sync()
    ///     .connection(TcpClient::new("localhost:5600").unwrap())
    ///     .conf();
    /// ```
    pub fn builder() -> NodeBuilder<Unset, Unset, Versionless, Unset, Unset> {
        NodeBuilder::new()
    }
}

impl<V: MaybeVersioned, C: MaybeConnConf> NodeConf<Edge<V>, V, C> {
    /// MAVLink system ID.
    #[inline(always)]
    pub fn system_id(&self) -> SystemId {
        self.kind.endpoint.system_id()
    }

    /// MAVLink component ID.
    #[inline(always)]
    pub fn component_id(&self) -> ComponentId {
        self.kind.endpoint.component_id()
    }
}

impl<K: NodeKind, V: Versioned, C: HasConnConf> NodeConf<K, V, C> {
    /// MAVLink version.
    #[inline(always)]
    pub fn version(&self) -> MavLinkVersion {
        V::version()
    }
}

impl<K: NodeKind, V: MaybeVersioned, C: HasConnConf> NodeConf<K, V, C> {
    /// Timeout for MAVLink heartbeats.
    ///
    /// If peer hasn't sent heartbeats for a specified period, it will be considered inactive.
    ///
    /// Default timeout is [`DEFAULT_HEARTBEAT_TIMEOUT`](crate::core::consts::DEFAULT_HEARTBEAT_TIMEOUT).
    #[inline(always)]
    pub fn heartbeat_timeout(&self) -> Duration {
        self.heartbeat_timeout
    }

    /// Dialect specification.
    ///
    /// Default dialect is [`DefaultDialect`].
    #[inline(always)]
    pub fn dialect(&self) -> &DialectSpec {
        self.dialects.main()
    }

    /// Known dialects specifications.
    ///
    /// Node can perform frame validation against known dialects. However, automatic operations,
    /// like heartbeats, will use the main [`NodeConf::dialect`].
    ///
    /// Main [`NodeConf::dialect`] is always among the known dialects.
    pub fn known_dialects(&self) -> impl Iterator<Item = &DialectSpec> {
        self.dialects.known()
    }

    /// Signature configuration.
    #[inline(always)]
    pub fn signer(&self) -> Option<&FrameSigner> {
        self.signer.as_ref()
    }

    /// Compatibility configuration.
    #[inline(always)]
    pub fn compat(&self) -> Option<&CompatProcessor> {
        self.compat.as_ref()
    }

    /// Returns `true` if it makes sense to restart the node after connection failure.
    pub fn is_repairable(&self) -> bool {
        self.connection_conf.is_repairable()
    }

    pub(crate) fn make_processor(&self) -> FrameProcessor {
        let mut builder = FrameProcessor::builder();

        if let Some(signer) = self.signer.clone() {
            builder = builder.signer(signer);
        }
        if let Some(compat) = self.compat {
            builder = builder.compat(compat);
        }

        builder
            .dialects(self.dialects.clone())
            .processors(self.processors.clone())
            .build()
    }
}

impl<V: Versioned, C: HasConnConf> NodeConf<Edge<V>, V, C> {
    /// Interval for MAVLink heartbeats.
    ///
    /// Node will send heartbeats within this interval.
    ///
    /// Default interval is [`DEFAULT_HEARTBEAT_INTERVAL`](crate::core::consts::DEFAULT_HEARTBEAT_INTERVAL).
    pub fn heartbeat_interval(&self) -> Duration {
        self.heartbeat_interval
    }
}

impl<K: NodeKind, V: MaybeVersioned, C: MaybeConnConf> NodeConf<K, V, C> {
    /// Converts arbitrary node configuration into a [`Proxy`] by stripping unnecessary information.
    ///
    /// This will set [`NodeConf::heartbeat_interval`] to the default value of the
    /// [`DEFAULT_HEARTBEAT_INTERVAL`].
    pub fn into_proxy(self) -> NodeConf<Proxy, V, C> {
        NodeConf {
            kind: Proxy,
            connection_conf: self.connection_conf,
            heartbeat_timeout: self.heartbeat_timeout,
            heartbeat_interval: DEFAULT_HEARTBEAT_INTERVAL,
            dialects: self.dialects,
            signer: self.signer,
            compat: self.compat,
            processors: self.processors,
            _version: self._version,
        }
    }
}

impl<K: NodeKind, V: MaybeVersioned, C: MaybeConnConf> IntoNodeConf<K, V, C> for NodeConf<K, V, C> {
    fn into_node_conf(self) -> NodeConf<K, V, C> {
        self
    }
}

#[cfg(test)]
#[cfg(feature = "sync")]
mod tests {
    use crate::core::io::ConnectionDetails;
    use crate::core::io::TcpClient;

    use super::*;

    #[test]
    fn node_conf_no_dialect_builder_workflow() {
        let node = NodeConf::builder()
            .sync()
            .system_id(10)
            .component_id(42)
            .connection(TcpClient::new("localhost:5600").unwrap())
            .conf();

        assert_eq!(node.system_id(), 10);
        assert_eq!(node.component_id(), 42);
    }

    #[test]
    fn node_conf_no_dialect_no_id_builder_workflow() {
        NodeConf::builder()
            .sync()
            .connection(TcpClient::new("localhost:5600").unwrap())
            .conf();
    }

    #[test]
    fn node_conf_builder_workflow() {
        let node_conf = NodeConf::builder()
            .sync()
            .system_id(10)
            .component_id(42)
            .connection(TcpClient::new("localhost:5600").unwrap())
            .conf();

        assert_eq!(node_conf.system_id(), 10);
        assert_eq!(node_conf.component_id(), 42);
        assert!(matches!(
            node_conf.connection().info().details(),
            ConnectionDetails::TcpClient { .. }
        ));
    }
}
