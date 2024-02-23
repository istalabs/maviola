//! MAVLink node configuration.

use std::marker::PhantomData;
use std::time::Duration;

use crate::core::marker::{
    HasComponentId, HasConnConf, HasSystemId, Identified, MaybeConnConf, MaybeIdentified,
    NoComponentId, NoConnConf, NoSystemId, Unidentified,
};
use crate::core::node::NodeBuilder;
use crate::protocol::{
    ComponentId, MavLinkVersion, MaybeVersioned, SystemId, Versioned, Versionless,
};

#[cfg(feature = "sync")]
use crate::sync::conn::ConnectionBuilder;
#[cfg(feature = "sync")]
use crate::sync::marker::ConnConf;
#[cfg(feature = "sync")]
use crate::sync::Node;

use crate::prelude::*;

/// MAVLink node configuration.
///
/// Node configuration can be instantiated only through [`NodeBuilder`]. Once node configuration is
/// obtained from [`NodeBuilder::conf`], it can be used to construct a node by [`NodeConf::build`]
/// or updated via [`NodeConf::update`]. The latter will turn node configuration to a
/// [`NodeBuilder`] populated with current settings.
///
/// The main reason to use [`NodeConf`] instead of directly creating a node is that
/// configurations are dormant and can be cloned. While nodes are dynamic and handle connection
/// context other runtime-specific entities that can't be cloned.
#[derive(Clone, Debug)]
pub struct NodeConf<I: MaybeIdentified, D: Dialect, V: MaybeVersioned, C: MaybeConnConf> {
    pub(crate) id: I,
    pub(crate) version: V,
    pub(crate) connection_conf: C,
    pub(crate) heartbeat_timeout: Duration,
    pub(crate) heartbeat_interval: Duration,
    pub(crate) _dialect: PhantomData<D>,
}

impl NodeConf<Unidentified, Minimal, Versionless, NoConnConf> {
    /// Creates an empty [`NodeBuilder`].
    ///
    /// # Usage
    ///
    /// Create node configuration that explicitly speaks `minimal` dialect.
    ///
    /// ```rust
    /// use maviola::core::NodeConf;
    /// use maviola::sync::TcpClient;
    /// use maviola::dialects::Minimal;
    ///
    /// let node = NodeConf::builder()
    ///     .system_id(10)
    ///     .component_id(42)
    ///     .connection(TcpClient::new("localhost:5600").unwrap())
    ///     .dialect::<Minimal>()
    ///     .conf();
    ///
    /// assert_eq!(node.system_id(), 10);
    /// assert_eq!(node.component_id(), 42);
    /// ```
    ///
    /// Create node configuration with default minimal dialect.
    ///
    /// ```rust
    /// use maviola::core::NodeConf;
    /// use maviola::sync::TcpClient;
    ///
    /// let node = NodeConf::builder()
    ///     .system_id(10)
    ///     .component_id(42)
    ///     .connection(TcpClient::new("localhost:5600").unwrap())
    ///     .conf();
    ///
    /// assert_eq!(node.system_id(), 10);
    /// assert_eq!(node.component_id(), 42);
    /// ```
    ///
    /// Create a configuration for unidentified node with default minimal dialect.
    ///
    /// ```rust
    /// use maviola::core::NodeConf;
    /// use maviola::sync::TcpClient;
    ///
    /// let node = NodeConf::builder()
    ///     .connection(TcpClient::new("localhost:5600").unwrap())
    ///     .conf();
    /// ```
    pub fn builder() -> NodeBuilder<NoSystemId, NoComponentId, Minimal, Versionless, NoConnConf> {
        NodeBuilder::new()
    }
}

impl<D: Dialect, V: MaybeVersioned, C: HasConnConf> NodeConf<Identified, D, V, C> {
    /// MAVLink system ID.
    pub fn system_id(&self) -> SystemId {
        self.id.system_id
    }

    /// MAVLink component ID.
    pub fn component_id(&self) -> ComponentId {
        self.id.component_id
    }
}

#[cfg(feature = "sync")]
impl<I: MaybeIdentified, D: Dialect, V: MaybeVersioned> NodeConf<I, D, V, ConnConf<V>> {
    /// Synchronous connection configuration.
    pub fn connection(&self) -> &dyn ConnectionBuilder<V> {
        self.connection_conf.0.as_ref()
    }
}

impl<I: MaybeIdentified, D: Dialect, V: Versioned, C: HasConnConf> NodeConf<I, D, V, C> {
    /// MAVLink version.
    pub fn version(&self) -> MavLinkVersion {
        V::version()
    }
}

impl<I: MaybeIdentified, D: Dialect, V: MaybeVersioned, C: HasConnConf> NodeConf<I, D, V, C> {
    /// Timeout for MAVLink heartbeats.
    ///
    /// If peer hasn't been sent heartbeats for as long as specified duration, it will be considered
    /// inactive.
    ///
    /// Default timeout is [`DEFAULT_HEARTBEAT_TIMEOUT`](crate::core::consts::DEFAULT_HEARTBEAT_TIMEOUT).
    pub fn heartbeat_timeout(&self) -> Duration {
        self.heartbeat_timeout
    }
}

impl<D: Dialect, V: Versioned, C: HasConnConf> NodeConf<Identified, D, V, C> {
    /// Interval for MAVLink heartbeats.
    ///
    /// Node will send heartbeats within this interval.
    ///
    /// Default interval is [`DEFAULT_HEARTBEAT_INTERVAL`](crate::core::consts::DEFAULT_HEARTBEAT_INTERVAL).
    pub fn heartbeat_interval(&self) -> Duration {
        self.heartbeat_interval
    }
}

impl<D: Dialect, V: MaybeVersioned, C: HasConnConf> NodeConf<Identified, D, V, C> {
    /// Creates a [`NodeBuilder`] initialised with current configuration.
    pub fn update(self) -> NodeBuilder<HasSystemId, HasComponentId, D, V, C> {
        NodeBuilder {
            system_id: HasSystemId(self.id.system_id),
            component_id: HasComponentId(self.id.component_id),
            version: self.version,
            conn_conf: self.connection_conf,
            heartbeat_timeout: self.heartbeat_timeout,
            heartbeat_interval: self.heartbeat_timeout,
            _dialect: self._dialect,
        }
    }
}

impl<D: Dialect, V: MaybeVersioned, C: HasConnConf> NodeConf<Unidentified, D, V, C> {
    /// Creates a [`NodeBuilder`] initialised with current configuration.
    pub fn update(self) -> NodeBuilder<NoSystemId, NoComponentId, D, V, C> {
        NodeBuilder {
            system_id: NoSystemId,
            component_id: NoComponentId,
            version: self.version,
            conn_conf: self.connection_conf,
            heartbeat_timeout: self.heartbeat_timeout,
            heartbeat_interval: self.heartbeat_timeout,
            _dialect: self._dialect,
        }
    }
}

impl<I: MaybeIdentified, D: Dialect, V: MaybeVersioned> NodeConf<I, D, V, ConnConf<V>> {
    /// Creates a [`Node`] initialised with current configuration.
    pub fn build(self) -> Result<Node<I, D, V>> {
        Node::try_from_conf(self)
    }
}

#[cfg(test)]
mod tests {
    use crate::core::io::ConnectionInfo;
    use crate::sync::TcpClient;

    use super::*;

    #[test]
    fn node_conf_no_dialect_builder_workflow() {
        let node = NodeConf::builder()
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
            .connection(TcpClient::new("localhost:5600").unwrap())
            .conf();
    }

    #[test]
    fn node_conf_builder_workflow() {
        let node_conf = NodeConf::builder()
            .system_id(10)
            .component_id(42)
            .dialect::<Minimal>()
            .connection(TcpClient::new("localhost:5600").unwrap())
            .conf();

        assert_eq!(node_conf.system_id(), 10);
        assert_eq!(node_conf.component_id(), 42);
        assert!(matches!(
            node_conf.connection().info(),
            ConnectionInfo::TcpClient { .. }
        ));
    }
}
