//! MAVLink node configuration.

use mavio::protocol::{DialectImpl, DialectMessage, MavLinkVersion};

use crate::io::node_variants::{Identified, IsIdentified, NotIdentified};
use crate::io::sync::ConnectionConf;
use crate::protocol::variants::{
    HasDialect, IsDialect, IsVersioned, NoDialect, NotVersioned, Versioned,
};

/// MAVLink node configuration.
///
/// Node configuration can be instantiated only through [`NodeConfBuilder`](builder::NodeConfBuilder).
#[derive(Debug)]
pub struct NodeConf<I: IsIdentified, D: IsDialect, V: IsVersioned> {
    pub(crate) id: I,
    pub(crate) dialect: D,
    pub(crate) version: V,
    conn_conf: Box<dyn ConnectionConf>,
}

impl<D: IsDialect, V: IsVersioned> NodeConf<Identified, D, V> {
    /// MAVLink system ID.
    pub fn system_id(&self) -> u8 {
        self.id.system_id
    }

    /// MAVLink component ID.
    pub fn component_id(&self) -> u8 {
        self.id.component_id
    }
}

impl NodeConf<NotIdentified, NoDialect, NotVersioned> {
    /// Creates an empty [`NodeBuilder`](builder::NodeConfBuilder).
    ///
    /// # Usage
    ///
    /// Create node configuration that speaks `minimal` dialect.
    ///
    /// ```rust
    /// use maviola::io::node_conf::NodeConf;
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
    /// use maviola::io::node_conf::NodeConf;
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
    /// use maviola::io::node_conf::NodeConf;
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
        NoDialect,
        NotVersioned,
    > {
        builder::NodeConfBuilder::new()
    }
}

impl<I: IsIdentified, V: IsVersioned, M: DialectMessage> NodeConf<I, HasDialect<M>, V> {
    /// MAVLink dialect.
    pub fn dialect(&self) -> &'static dyn DialectImpl<Message = M> {
        self.dialect.0
    }
}

impl<I: IsIdentified, D: IsDialect, V: IsVersioned> NodeConf<I, D, V> {
    /// Connection configuration.
    pub fn conn_conf(&self) -> &dyn ConnectionConf {
        self.conn_conf.as_ref()
    }
}

impl<I: IsIdentified, D: IsDialect, V: Versioned> NodeConf<I, D, V> {
    /// MAVLink version.
    pub fn version(&self) -> MavLinkVersion {
        self.version.mavlink_version()
    }
}

/// Builder for [`NodeConf`].
pub mod builder {
    use mavio::protocol::{DialectImpl, DialectMessage};

    use crate::io::sync::ConnectionConf;
    use crate::protocol::variants::{IsVersioned, MavLink1, MavLink2, NotVersioned};

    use super::NodeConf;
    use super::{HasDialect, Identified, IsDialect, NoDialect, NotIdentified};

    /// Marker trait for [`NodeConfBuilder`] with or without [`NodeConf::system_id`].
    pub trait IsSystemId {}

    /// Marker for [`NodeConfBuilder`] without [`NodeConf::system_id`].
    pub struct NoSystemId();
    impl IsSystemId for NoSystemId {}

    /// Marker for [`NodeConfBuilder`] with [`NodeConf::system_id`] set.
    pub struct SystemId(u8);
    impl IsSystemId for SystemId {}

    /// Marker trait for [`NodeConfBuilder`] with or without [`NodeConf::component_id`].
    pub trait IsComponentId {}

    /// Marker for [`NodeConfBuilder`] without [`NodeConf::component_id`].
    pub struct NoComponentId();
    impl IsComponentId for NoComponentId {}

    /// Marker for [`NodeConfBuilder`] with [`NodeConf::component_id`] set.
    pub struct ComponentId(u8);
    impl IsComponentId for ComponentId {}

    /// Marker trait for [`NodeConfBuilder`] with or without [`NodeConf::conn_conf`].
    pub trait IsConnConf {}

    /// Marker for [`NodeConfBuilder`] without [`NodeConf::conn_conf`].
    pub struct NoConnConf();
    impl IsConnConf for NoConnConf {}

    /// Marker for [`NodeConfBuilder`] with [`NodeConf::conn_conf`] set.
    pub struct ConnConf(Box<dyn ConnectionConf>);
    impl IsConnConf for ConnConf {}

    /// Builder for [`NodeConf`].
    #[derive(Clone, Debug, Default)]
    pub struct NodeConfBuilder<
        S: IsSystemId,
        C: IsComponentId,
        CC: IsConnConf,
        D: IsDialect,
        V: IsVersioned,
    > {
        system_id: S,
        component_id: C,
        dialect: D,
        conn_conf: CC,
        version: V,
    }

    impl NodeConfBuilder<NoSystemId, NoComponentId, NoConnConf, NoDialect, NotVersioned> {
        /// Instantiates an empty [`NodeConfBuilder`].
        pub fn new() -> Self {
            Self {
                system_id: NoSystemId(),
                component_id: NoComponentId(),
                dialect: NoDialect(),
                conn_conf: NoConnConf(),
                version: NotVersioned(),
            }
        }
    }

    impl<C: IsComponentId, CC: IsConnConf, D: IsDialect, V: IsVersioned>
        NodeConfBuilder<NoSystemId, C, CC, D, V>
    {
        /// Sets [`NodeConf::system_id`].
        pub fn system_id(self, system_id: u8) -> NodeConfBuilder<SystemId, C, CC, D, V> {
            NodeConfBuilder {
                system_id: SystemId(system_id),
                component_id: self.component_id,
                dialect: self.dialect,
                conn_conf: self.conn_conf,
                version: self.version,
            }
        }
    }

    impl<S: IsSystemId, T: IsConnConf, D: IsDialect, V: IsVersioned>
        NodeConfBuilder<S, NoComponentId, T, D, V>
    {
        /// Sets [`NodeConf::component_id`].
        pub fn component_id(self, component_id: u8) -> NodeConfBuilder<S, ComponentId, T, D, V> {
            NodeConfBuilder {
                system_id: self.system_id,
                component_id: ComponentId(component_id),
                dialect: self.dialect,
                conn_conf: self.conn_conf,
                version: self.version,
            }
        }
    }

    impl<S: IsSystemId, C: IsComponentId, D: IsDialect, V: IsVersioned>
        NodeConfBuilder<S, C, NoConnConf, D, V>
    {
        /// Sets [`NodeConf::component_id`].
        pub fn conn_conf(
            self,
            conn_conf: impl ConnectionConf + 'static,
        ) -> NodeConfBuilder<S, C, ConnConf, D, V> {
            NodeConfBuilder {
                system_id: self.system_id,
                component_id: self.component_id,
                dialect: self.dialect,
                conn_conf: ConnConf(Box::new(conn_conf)),
                version: self.version,
            }
        }
    }

    impl<S: IsSystemId, C: IsComponentId, CC: IsConnConf, V: IsVersioned>
        NodeConfBuilder<S, C, CC, NoDialect, V>
    {
        /// Sets [`NodeConf::dialect`].
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
            }
        }
    }

    impl<S: IsSystemId, C: IsComponentId, CC: IsConnConf, D: IsDialect>
        NodeConfBuilder<S, C, CC, D, NotVersioned>
    {
        /// Sets [`NodeConf::dialect`].
        pub fn v1(self) -> NodeConfBuilder<S, C, CC, D, MavLink1> {
            NodeConfBuilder {
                system_id: self.system_id,
                component_id: self.component_id,
                dialect: self.dialect,
                conn_conf: self.conn_conf,
                version: MavLink1(),
            }
        }
    }

    impl<S: IsSystemId, C: IsComponentId, CC: IsConnConf, D: IsDialect>
        NodeConfBuilder<S, C, CC, D, NotVersioned>
    {
        /// Sets [`NodeConf::dialect`].
        pub fn v2(self) -> NodeConfBuilder<S, C, CC, D, MavLink2> {
            NodeConfBuilder {
                system_id: self.system_id,
                component_id: self.component_id,
                dialect: self.dialect,
                conn_conf: self.conn_conf,
                version: MavLink2(),
            }
        }
    }

    impl<D: IsDialect, V: IsVersioned> NodeConfBuilder<NoSystemId, NoComponentId, ConnConf, D, V> {
        /// Builds and instance of [`NodeConf`] without defined [`NodeConf::system_id`] and [`NodeConf::component_id`].
        pub fn build(self) -> NodeConf<NotIdentified, D, V> {
            NodeConf {
                id: NotIdentified(),
                dialect: self.dialect,
                conn_conf: self.conn_conf.0,
                version: self.version,
            }
        }
    }

    impl<D: IsDialect, V: IsVersioned> NodeConfBuilder<SystemId, ComponentId, ConnConf, D, V> {
        /// Builds and instance of [`NodeConf`] with defined [`NodeConf::system_id`] and [`NodeConf::component_id`].
        pub fn build(self) -> NodeConf<Identified, D, V> {
            NodeConf {
                id: Identified {
                    system_id: self.system_id.0,
                    component_id: self.component_id.0,
                },
                dialect: self.dialect,
                conn_conf: self.conn_conf.0,
                version: self.version,
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
        if let minimal::Message::Heartbeat(_) = decoded_message {
            // Message was correctly decoded back from payload
        } else {
            panic!("Invalid decoded message: {decoded_message:#?}");
        }
    }
}
