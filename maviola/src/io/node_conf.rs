//! MAVLink node configuration.

use std::time::Duration;

use mavio::protocol::{
    ComponentId, DialectImpl, DialectMessage, MavLinkVersion, MaybeVersioned, SystemId, Versioned,
    Versionless,
};

use crate::io::sync::connection::ConnectionConf;
use crate::marker::{
    Dialectless, HasDialect, Identified, IsIdentified, MaybeDialect, NotIdentified,
};

/// MAVLink node configuration.
///
/// Node configuration can be instantiated only through [`NodeConfBuilder`](builder::NodeConfBuilder).
#[derive(Debug)]
pub struct NodeConf<I: IsIdentified, D: MaybeDialect, V: MaybeVersioned> {
    pub(crate) id: I,
    pub(crate) dialect: D,
    pub(crate) version: V,
    conn_conf: Box<dyn ConnectionConf<V>>,
    pub(crate) heartbeat_timeout: Duration,
    pub(crate) heartbeat_interval: Duration,
}

impl<D: MaybeDialect, V: MaybeVersioned> NodeConf<Identified, D, V> {
    /// MAVLink system ID.
    pub fn system_id(&self) -> SystemId {
        self.id.system_id
    }

    /// MAVLink component ID.
    pub fn component_id(&self) -> ComponentId {
        self.id.component_id
    }
}

impl NodeConf<NotIdentified, Dialectless, Versionless> {
    /// Creates an empty [`NodeBuilder`](builder::NodeConfBuilder).
    ///
    /// # Usage
    ///
    /// Create node configuration that speaks `minimal` dialect.
    ///
    /// ```rust
    /// use maviola::io::NodeConf;
    /// use maviola::io::sync::TcpClientConf;
    /// use maviola::dialects::minimal;
    ///
    /// let node = NodeConf::builder()
    ///     .system_id(10)
    ///     .component_id(42)
    ///     .conn_conf(TcpClientConf::new("localhost:5600").unwrap())
    ///     .dialect(minimal::dialect()).build();
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
    /// use maviola::io::sync::TcpClientConf;
    ///
    /// let node = NodeConf::builder()
    ///     .system_id(10)
    ///     .component_id(42)
    ///     .conn_conf(TcpClientConf::new("localhost:5600").unwrap())
    ///     .build();
    ///
    /// assert_eq!(node.system_id(), 10);
    /// assert_eq!(node.component_id(), 42);
    /// ```
    ///
    /// Create a configuration for unidentified node without a specific dialect.
    ///
    /// ```rust
    /// use maviola::io::NodeConf;
    /// use maviola::io::sync::TcpClientConf;
    ///
    /// let node = NodeConf::builder()
    ///     .conn_conf(TcpClientConf::new("localhost:5600").unwrap())
    ///     .build();
    /// ```
    pub fn builder() -> builder::NodeConfBuilder<
        builder::NoSystemId,
        builder::NoComponentId,
        builder::NoConnConf,
        Dialectless,
        Versionless,
    > {
        builder::NodeConfBuilder::new()
    }
}

impl<I: IsIdentified, V: MaybeVersioned, M: DialectMessage> NodeConf<I, HasDialect<M>, V> {
    /// MAVLink dialect.
    pub fn dialect(&self) -> &'static dyn DialectImpl<Message = M> {
        self.dialect.0
    }
}

impl<I: IsIdentified, D: MaybeDialect, V: MaybeVersioned> NodeConf<I, D, V> {
    /// Connection configuration.
    pub fn conn_conf(&self) -> &dyn ConnectionConf<V> {
        self.conn_conf.as_ref()
    }
}

impl<I: IsIdentified, D: MaybeDialect, V: Versioned> NodeConf<I, D, V> {
    /// MAVLink version.
    pub fn version(&self) -> MavLinkVersion {
        V::version()
    }
}

impl<I: IsIdentified, D: MaybeDialect, V: MaybeVersioned> NodeConf<I, D, V> {
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

impl<V: Versioned, M: DialectMessage> NodeConf<Identified, HasDialect<M>, V> {
    /// Interval for MAVLink heartbeats.
    ///
    /// Node will send heartbeats within this interval.
    ///
    /// Default interval is [`DEFAULT_HEARTBEAT_INTERVAL`](crate::consts::DEFAULT_HEARTBEAT_INTERVAL).
    pub fn heartbeat_interval(&self) -> Duration {
        self.heartbeat_interval
    }
}

/// Builder for [`NodeConf`].
pub mod builder {
    use std::time::Duration;

    use mavio::protocol::{
        ComponentId, DialectImpl, DialectMessage, MaybeVersioned, SystemId, Versioned, Versionless,
    };

    use crate::consts::{DEFAULT_HEARTBEAT_INTERVAL, DEFAULT_HEARTBEAT_TIMEOUT};
    use crate::io::sync::connection::ConnectionConf;
    use crate::marker::{Dialectless, HasDialect};

    use super::NodeConf;
    use super::{Identified, MaybeDialect, NotIdentified};

    /// Marker trait for [`NodeConfBuilder`] with or without [`NodeConf::system_id`].
    pub trait IsSystemId {}

    /// Marker for [`NodeConfBuilder`] without [`NodeConf::system_id`].
    pub struct NoSystemId();
    impl IsSystemId for NoSystemId {}

    /// Marker for [`NodeConfBuilder`] with [`NodeConf::system_id`] set.
    pub struct HasSystemId(u8);
    impl IsSystemId for HasSystemId {}

    /// Marker trait for [`NodeConfBuilder`] with or without [`NodeConf::component_id`].
    pub trait IsComponentId {}

    /// Marker for [`NodeConfBuilder`] without [`NodeConf::component_id`].
    pub struct NoComponentId();
    impl IsComponentId for NoComponentId {}

    /// Marker for [`NodeConfBuilder`] with [`NodeConf::component_id`] set.
    pub struct HasComponentId(u8);
    impl IsComponentId for HasComponentId {}

    /// Marker trait for [`NodeConfBuilder`] with or without [`NodeConf::conn_conf`].
    pub trait IsConnConf {}

    /// Marker for [`NodeConfBuilder`] without [`NodeConf::conn_conf`].
    pub struct NoConnConf();
    impl IsConnConf for NoConnConf {}

    /// Marker for [`NodeConfBuilder`] with [`NodeConf::conn_conf`] set.
    pub struct ConnConf<V: MaybeVersioned>(Box<dyn ConnectionConf<V>>);
    impl<V: MaybeVersioned> IsConnConf for ConnConf<V> {}

    /// Builder for [`NodeConf`].
    #[derive(Clone, Debug, Default)]
    pub struct NodeConfBuilder<
        S: IsSystemId,
        C: IsComponentId,
        CC: IsConnConf,
        D: MaybeDialect,
        V: MaybeVersioned,
    > {
        system_id: S,
        component_id: C,
        dialect: D,
        conn_conf: CC,
        version: V,
        heartbeat_timeout: Duration,
        heartbeat_interval: Duration,
    }

    impl NodeConfBuilder<NoSystemId, NoComponentId, NoConnConf, Dialectless, Versionless> {
        /// Instantiate an empty [`NodeConfBuilder`].
        pub fn new() -> Self {
            Self {
                system_id: NoSystemId(),
                component_id: NoComponentId(),
                dialect: Dialectless,
                conn_conf: NoConnConf(),
                version: Versionless,
                heartbeat_timeout: DEFAULT_HEARTBEAT_TIMEOUT,
                heartbeat_interval: DEFAULT_HEARTBEAT_INTERVAL,
            }
        }
    }

    impl<C: IsComponentId, CC: IsConnConf, D: MaybeDialect, V: MaybeVersioned>
        NodeConfBuilder<NoSystemId, C, CC, D, V>
    {
        /// Set [`NodeConf::system_id`].
        pub fn system_id(self, system_id: SystemId) -> NodeConfBuilder<HasSystemId, C, CC, D, V> {
            NodeConfBuilder {
                system_id: HasSystemId(system_id),
                component_id: self.component_id,
                dialect: self.dialect,
                conn_conf: self.conn_conf,
                version: self.version,
                heartbeat_timeout: self.heartbeat_timeout,
                heartbeat_interval: self.heartbeat_interval,
            }
        }
    }

    impl<S: IsSystemId, T: IsConnConf, D: MaybeDialect, V: MaybeVersioned>
        NodeConfBuilder<S, NoComponentId, T, D, V>
    {
        /// Set [`NodeConf::component_id`].
        pub fn component_id(
            self,
            component_id: ComponentId,
        ) -> NodeConfBuilder<S, HasComponentId, T, D, V> {
            NodeConfBuilder {
                system_id: self.system_id,
                component_id: HasComponentId(component_id),
                dialect: self.dialect,
                conn_conf: self.conn_conf,
                version: self.version,
                heartbeat_timeout: self.heartbeat_timeout,
                heartbeat_interval: self.heartbeat_interval,
            }
        }
    }

    impl<S: IsSystemId, C: IsComponentId, CC: IsConnConf, D: MaybeDialect, V: MaybeVersioned>
        NodeConfBuilder<S, C, CC, D, V>
    {
        /// Set [`NodeConf::heartbeat_timeout`].
        pub fn heartbeat_timeout(
            self,
            heartbeat_timeout: Duration,
        ) -> NodeConfBuilder<S, C, CC, D, V> {
            NodeConfBuilder {
                system_id: self.system_id,
                component_id: self.component_id,
                dialect: self.dialect,
                conn_conf: self.conn_conf,
                version: self.version,
                heartbeat_timeout,
                heartbeat_interval: self.heartbeat_interval,
            }
        }
    }

    impl<S: IsSystemId, C: IsComponentId, D: MaybeDialect, V: MaybeVersioned>
        NodeConfBuilder<S, C, NoConnConf, D, V>
    {
        /// Set [`NodeConf::component_id`].
        pub fn conn_conf(
            self,
            conn_conf: impl ConnectionConf<V> + 'static,
        ) -> NodeConfBuilder<S, C, ConnConf<V>, D, V> {
            NodeConfBuilder {
                system_id: self.system_id,
                component_id: self.component_id,
                dialect: self.dialect,
                conn_conf: ConnConf(Box::new(conn_conf)),
                version: self.version,
                heartbeat_timeout: self.heartbeat_timeout,
                heartbeat_interval: self.heartbeat_interval,
            }
        }
    }

    impl<S: IsSystemId, C: IsComponentId, CC: IsConnConf, V: MaybeVersioned>
        NodeConfBuilder<S, C, CC, Dialectless, V>
    {
        /// Set [`NodeConf::dialect`].
        pub fn dialect<M: DialectMessage>(
            self,
            dialect: &'static dyn DialectImpl<Message = M>,
        ) -> NodeConfBuilder<S, C, CC, HasDialect<M>, V> {
            NodeConfBuilder {
                system_id: self.system_id,
                component_id: self.component_id,
                dialect: HasDialect(dialect),
                conn_conf: self.conn_conf,
                version: self.version,
                heartbeat_timeout: self.heartbeat_timeout,
                heartbeat_interval: self.heartbeat_interval,
            }
        }
    }

    impl<S: IsSystemId, C: IsComponentId, CC: IsConnConf, D: MaybeDialect>
        NodeConfBuilder<S, C, CC, D, Versionless>
    {
        /// Set [`NodeConf::dialect`].
        pub fn version<Version: Versioned>(
            self,
            version: Version,
        ) -> NodeConfBuilder<S, C, CC, D, Version> {
            NodeConfBuilder {
                system_id: self.system_id,
                component_id: self.component_id,
                dialect: self.dialect,
                conn_conf: self.conn_conf,
                version,
                heartbeat_timeout: self.heartbeat_timeout,
                heartbeat_interval: self.heartbeat_interval,
            }
        }
    }

    impl<CC: IsConnConf, V: Versioned, M: DialectMessage>
        NodeConfBuilder<HasSystemId, HasComponentId, CC, HasDialect<M>, V>
    {
        /// Set [`NodeConf::heartbeat_interval`].
        ///
        /// This parameter makes sense only for nodes that are identified, has a specified dialect
        /// and MAVLink protocol version. Therefore, the method is available only when the following
        /// parameters have been already set:
        ///
        /// * [`system_id`](NodeConfBuilder::system_id)
        /// * [`component_id`](NodeConfBuilder::component_id)
        /// * [`dialect`](NodeConfBuilder::dialect)
        /// * [`version`](NodeConfBuilder::version)
        pub fn heartbeat_interval(
            self,
            heartbeat_interval: Duration,
        ) -> NodeConfBuilder<HasSystemId, HasComponentId, CC, HasDialect<M>, V> {
            NodeConfBuilder {
                system_id: self.system_id,
                component_id: self.component_id,
                dialect: self.dialect,
                conn_conf: self.conn_conf,
                version: self.version,
                heartbeat_timeout: self.heartbeat_timeout,
                heartbeat_interval,
            }
        }
    }

    impl<D: MaybeDialect, V: MaybeVersioned>
        NodeConfBuilder<NoSystemId, NoComponentId, ConnConf<V>, D, V>
    {
        /// Build and instance of [`NodeConf`] without defined [`NodeConf::system_id`] and
        /// [`NodeConf::component_id`].
        pub fn build(self) -> NodeConf<NotIdentified, D, V> {
            NodeConf {
                id: NotIdentified,
                dialect: self.dialect,
                conn_conf: self.conn_conf.0,
                version: self.version,
                heartbeat_timeout: self.heartbeat_timeout,
                heartbeat_interval: self.heartbeat_interval,
            }
        }
    }

    impl<D: MaybeDialect, V: MaybeVersioned>
        NodeConfBuilder<HasSystemId, HasComponentId, ConnConf<V>, D, V>
    {
        /// Build and instance of [`NodeConf`] with defined [`NodeConf::system_id`] and
        /// [`NodeConf::component_id`].
        pub fn build(self) -> NodeConf<Identified, D, V> {
            NodeConf {
                id: Identified {
                    system_id: self.system_id.0,
                    component_id: self.component_id.0,
                },
                dialect: self.dialect,
                conn_conf: self.conn_conf.0,
                version: self.version,
                heartbeat_timeout: self.heartbeat_timeout,
                heartbeat_interval: self.heartbeat_interval,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use mavio::protocol::MavLinkVersion;

    use crate::dialects::minimal;
    use crate::io::sync::TcpClientConf;

    use super::*;

    #[test]
    fn node_conf_no_dialect_builder_workflow() {
        let node = NodeConf::builder()
            .system_id(10)
            .component_id(42)
            .conn_conf(TcpClientConf::new("localhost:5600").unwrap())
            .build();

        assert_eq!(node.system_id(), 10);
        assert_eq!(node.component_id(), 42);
    }

    #[test]
    fn node_conf_no_dialect_no_id_builder_workflow() {
        NodeConf::builder()
            .conn_conf(TcpClientConf::new("localhost:5600").unwrap())
            .build();
    }

    #[test]
    fn node_conf_builder_workflow() {
        let node = NodeConf::builder()
            .system_id(10)
            .component_id(42)
            .dialect(minimal::dialect())
            .conn_conf(TcpClientConf::new("localhost:5600").unwrap())
            .build();

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
            .conn_conf(TcpClientConf::new("localhost:5600").unwrap())
            .build();

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
