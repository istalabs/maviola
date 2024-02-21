//! MAVLink node configuration.

use std::time::Duration;

use crate::io::marker::{
    HasComponentId, HasConnConf, HasSystemId, Identified, MaybeConnConf, MaybeIdentified,
    NoComponentId, NoConnConf, NoSystemId, Unidentified,
};
use crate::io::NodeBuilder;
use mavio::protocol::{
    ComponentId, DialectImpl, DialectMessage, MavLinkVersion, MaybeVersioned, SystemId, Versioned,
    Versionless,
};

#[cfg(feature = "sync")]
use crate::io::sync::connection::ConnectionConf;
#[cfg(feature = "sync")]
use crate::io::sync::marker::ConnConf;
use crate::protocol::{Dialectless, HasDialect, MaybeDialect};
use crate::Node;

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
pub struct NodeConf<I: MaybeIdentified, D: MaybeDialect, V: MaybeVersioned, C: MaybeConnConf> {
    pub(crate) id: I,
    pub(crate) dialect: D,
    pub(crate) version: V,
    pub(crate) connection_conf: C,
    pub(crate) heartbeat_timeout: Duration,
    pub(crate) heartbeat_interval: Duration,
}

impl NodeConf<Unidentified, Dialectless, Versionless, NoConnConf> {
    /// Creates an empty [`NodeBuilder`].
    ///
    /// # Usage
    ///
    /// Create node configuration that speaks `minimal` dialect.
    ///
    /// ```rust
    /// use maviola::io::NodeConf;
    /// use maviola::io::sync::TcpClient;
    /// use maviola::dialects::minimal;
    ///
    /// let node = NodeConf::builder()
    ///     .system_id(10)
    ///     .component_id(42)
    ///     .connection(TcpClient::new("localhost:5600").unwrap())
    ///     .dialect(minimal::dialect())
    ///     .conf();
    ///
    /// assert_eq!(node.system_id(), 10);
    /// assert_eq!(node.component_id(), 42);
    /// assert_eq!(node.dialect().name(), "minimal");
    /// ```
    ///
    /// Create node configuration without any dialect.
    ///
    /// ```rust
    /// use maviola::io::NodeConf;
    /// use maviola::io::sync::TcpClient;
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
    /// Create a configuration for unidentified node without a specific dialect.
    ///
    /// ```rust
    /// use maviola::io::NodeConf;
    /// use maviola::io::sync::TcpClient;
    ///
    /// let node = NodeConf::builder()
    ///     .connection(TcpClient::new("localhost:5600").unwrap())
    ///     .conf();
    /// ```
    pub fn builder() -> NodeBuilder<NoSystemId, NoComponentId, Dialectless, Versionless, NoConnConf>
    {
        NodeBuilder::new()
    }
}

impl<D: MaybeDialect, V: MaybeVersioned, C: HasConnConf> NodeConf<Identified, D, V, C> {
    /// MAVLink system ID.
    pub fn system_id(&self) -> SystemId {
        self.id.system_id
    }

    /// MAVLink component ID.
    pub fn component_id(&self) -> ComponentId {
        self.id.component_id
    }
}

impl<I: MaybeIdentified, V: MaybeVersioned, M: DialectMessage, C: HasConnConf>
    NodeConf<I, HasDialect<M>, V, C>
{
    /// MAVLink dialect.
    pub fn dialect(&self) -> &'static dyn DialectImpl<Message = M> {
        self.dialect.0
    }
}

#[cfg(feature = "sync")]
impl<I: MaybeIdentified, D: MaybeDialect, V: MaybeVersioned> NodeConf<I, D, V, ConnConf<V>> {
    /// Synchronous connection configuration.
    pub fn connection(&self) -> &dyn ConnectionConf<V> {
        self.connection_conf.0.as_ref()
    }
}

impl<I: MaybeIdentified, D: MaybeDialect, V: Versioned, C: HasConnConf> NodeConf<I, D, V, C> {
    /// MAVLink version.
    pub fn version(&self) -> MavLinkVersion {
        V::version()
    }
}

impl<I: MaybeIdentified, D: MaybeDialect, V: MaybeVersioned, C: HasConnConf> NodeConf<I, D, V, C> {
    /// Timeout for MAVLink heartbeats.
    ///
    /// If peer hasn't been sent heartbeats for as long as specified duration, it will be considered
    /// inactive.
    ///
    /// Default timeout is [`DEFAULT_HEARTBEAT_TIMEOUT`](crate::consts::DEFAULT_HEARTBEAT_TIMEOUT).
    pub fn heartbeat_timeout(&self) -> Duration {
        self.heartbeat_timeout
    }
}

impl<V: Versioned, M: DialectMessage, C: HasConnConf> NodeConf<Identified, HasDialect<M>, V, C> {
    /// Interval for MAVLink heartbeats.
    ///
    /// Node will send heartbeats within this interval.
    ///
    /// Default interval is [`DEFAULT_HEARTBEAT_INTERVAL`](crate::consts::DEFAULT_HEARTBEAT_INTERVAL).
    pub fn heartbeat_interval(&self) -> Duration {
        self.heartbeat_interval
    }
}

impl<D: MaybeDialect, V: MaybeVersioned, C: HasConnConf> NodeConf<Identified, D, V, C> {
    /// Creates a [`NodeBuilder`] initialised with current configuration.
    pub fn update(self) -> NodeBuilder<HasSystemId, HasComponentId, D, V, C> {
        NodeBuilder {
            system_id: HasSystemId(self.id.system_id),
            component_id: HasComponentId(self.id.component_id),
            dialect: self.dialect,
            version: self.version,
            conn_conf: self.connection_conf,
            heartbeat_timeout: self.heartbeat_timeout,
            heartbeat_interval: self.heartbeat_timeout,
        }
    }
}

impl<D: MaybeDialect, V: MaybeVersioned, C: HasConnConf> NodeConf<Unidentified, D, V, C> {
    /// Creates a [`NodeBuilder`] initialised with current configuration.
    pub fn update(self) -> NodeBuilder<NoSystemId, NoComponentId, D, V, C> {
        NodeBuilder {
            system_id: NoSystemId,
            component_id: NoComponentId,
            dialect: self.dialect,
            version: self.version,
            conn_conf: self.connection_conf,
            heartbeat_timeout: self.heartbeat_timeout,
            heartbeat_interval: self.heartbeat_timeout,
        }
    }
}

impl<I: MaybeIdentified, D: MaybeDialect, V: MaybeVersioned> NodeConf<I, D, V, ConnConf<V>> {
    /// Creates a [`Node`] initialised with current configuration.
    pub fn build(self) -> Result<Node<I, D, V>> {
        Node::try_from_conf(self)
    }
}

#[cfg(test)]
mod tests {
    use mavio::protocol::MavLinkVersion;

    use crate::dialects::minimal;
    use crate::io::sync::TcpClient;

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
        let node = NodeConf::builder()
            .system_id(10)
            .component_id(42)
            .dialect(minimal::dialect())
            .connection(TcpClient::new("localhost:5600").unwrap())
            .conf();

        assert_eq!(node.system_id(), 10);
        assert_eq!(node.component_id(), 42);
        assert_eq!(node.dialect().name(), "minimal");
    }

    #[test]
    fn node_conf_with_dialect_encode_decode() {
        let node = NodeConf::builder()
            .system_id(10)
            .component_id(42)
            .dialect(minimal::dialect())
            .connection(TcpClient::new("localhost:5600").unwrap())
            .conf();

        let message = minimal::messages::Heartbeat::default();

        let payload = node
            .dialect()
            .encode(&message.into(), MavLinkVersion::V2)
            .unwrap();

        let decoded_message = node.dialect().decode(&payload).unwrap();
        if let minimal::Minimal::Heartbeat(_) = decoded_message {
            // Message was correctly decoded back from payload
        } else {
            panic!("Invalid decoded message: {decoded_message:#?}");
        }
    }
}
