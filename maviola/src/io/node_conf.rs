//! MAVLink node configuration.

use std::marker::PhantomData;

use crate::io::node_variants::{
    Dialect, HasDialect, HasIdentifier, HasVersion, Identified, NoDialect, NotIdentified,
    NotVersioned, Versioned,
};
use mavio::protocol::{DialectImpl, DialectMessage, MavLinkVersion};

use crate::io::sync::ConnectionConf;

/// MAVLink node configuration.
///
/// Node configuration can be instantiated only through [`NodeConfBuilder`](builder::NodeConfBuilder).
#[derive(Debug)]
pub struct NodeConf<I: HasIdentifier, D: HasDialect, V: HasVersion, M: DialectMessage + 'static> {
    pub(crate) id: I,
    pub(crate) dialect: D,
    pub(crate) version: V,
    conn_conf: Box<dyn ConnectionConf>,
    _marker_message: PhantomData<M>,
}

impl<D: HasDialect, V: HasVersion, M: DialectMessage> NodeConf<Identified, D, V, M> {
    /// MAVLink system ID.
    pub fn system_id(&self) -> u8 {
        self.id.system_id
    }

    /// MAVLink component ID.
    pub fn component_id(&self) -> u8 {
        self.id.component_id
    }
}

impl NodeConf<NotIdentified, NoDialect, NotVersioned, crate::dialects::minimal::Message> {
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
    ///     .set_system_id(10)
    ///     .set_component_id(42)
    ///     .set_conn_conf(TcpClientConf::new("localhost:5600").unwrap())
    ///     .set_dialect(minimal::dialect()).build();
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
    ///     .set_system_id(10)
    ///     .set_component_id(42)
    ///     .set_conn_conf(TcpClientConf::new("localhost:5600").unwrap())
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
    ///     .set_conn_conf(TcpClientConf::new("localhost:5600").unwrap())
    ///     .build();
    /// ```
    pub fn builder() -> builder::NodeConfBuilder<
        builder::NoSystemId,
        builder::NoComponentId,
        builder::NoConnConf,
        NoDialect,
        NotVersioned,
        crate::dialects::minimal::Message,
    > {
        builder::NodeConfBuilder::new()
    }
}

impl<I: HasIdentifier, V: HasVersion, M: DialectMessage> NodeConf<I, Dialect<M>, V, M> {
    /// MAVLink dialect.
    pub fn dialect(&self) -> &'static dyn DialectImpl<Message = M> {
        self.dialect.0
    }
}

impl<I: HasIdentifier, D: HasDialect, V: HasVersion, M: DialectMessage> NodeConf<I, D, V, M> {
    /// Connection configuration.
    pub fn conn_conf(&self) -> &dyn ConnectionConf {
        self.conn_conf.as_ref()
    }
}

impl<I: HasIdentifier, D: HasDialect, M: DialectMessage> NodeConf<I, D, Versioned, M> {
    /// MAVLink version.
    pub fn version(&self) -> &MavLinkVersion {
        &self.version.0
    }
}

/// Builder for [`NodeConf`].
pub mod builder {
    use std::marker::PhantomData;

    use crate::io::node_variants::{HasVersion, NotVersioned, Versioned};
    use mavio::protocol::{DialectImpl, DialectMessage, MavLinkVersion};

    use crate::io::sync::ConnectionConf;

    use super::NodeConf;
    use super::{Dialect, HasDialect, Identified, NoDialect, NotIdentified};

    /// Marker trait for [`NodeConfBuilder`] with or without [`NodeConf::system_id`].
    pub trait HasSystemId {}

    /// Marker for [`NodeConfBuilder`] without [`NodeConf::system_id`].
    pub struct NoSystemId();
    impl HasSystemId for NoSystemId {}

    /// Marker for [`NodeConfBuilder`] with [`NodeConf::system_id`] set.
    pub struct SystemId(u8);
    impl HasSystemId for SystemId {}

    /// Marker trait for [`NodeConfBuilder`] with or without [`NodeConf::component_id`].
    pub trait HasComponentId {}

    /// Marker for [`NodeConfBuilder`] without [`NodeConf::component_id`].
    pub struct NoComponentId();
    impl HasComponentId for NoComponentId {}

    /// Marker for [`NodeConfBuilder`] with [`NodeConf::component_id`] set.
    pub struct ComponentId(u8);
    impl HasComponentId for ComponentId {}

    /// Marker trait for [`NodeConfBuilder`] with or without [`NodeConf::conn_conf`].
    pub trait HasConnConf {}

    /// Marker for [`NodeConfBuilder`] without [`NodeConf::conn_conf`].
    pub struct NoConnConf();
    impl HasConnConf for NoConnConf {}

    /// Marker for [`NodeConfBuilder`] with [`NodeConf::conn_conf`] set.
    pub struct ConnConf(Box<dyn ConnectionConf>);
    impl HasConnConf for ConnConf {}

    /// Builder for [`NodeConf`].
    #[derive(Clone, Debug, Default)]
    pub struct NodeConfBuilder<
        S: HasSystemId,
        C: HasComponentId,
        CC: HasConnConf,
        D: HasDialect,
        V: HasVersion,
        M: DialectMessage + 'static,
    > {
        system_id: S,
        component_id: C,
        dialect: D,
        conn_conf: CC,
        version: V,
        _marker_message: PhantomData<M>,
    }

    impl<M: DialectMessage>
        NodeConfBuilder<NoSystemId, NoComponentId, NoConnConf, NoDialect, NotVersioned, M>
    {
        /// Instantiates an empty [`NodeConfBuilder`].
        pub fn new() -> Self {
            Self {
                system_id: NoSystemId(),
                component_id: NoComponentId(),
                dialect: NoDialect(),
                conn_conf: NoConnConf(),
                version: NotVersioned(),
                _marker_message: Default::default(),
            }
        }
    }

    impl<C: HasComponentId, CC: HasConnConf, D: HasDialect, V: HasVersion, M: DialectMessage>
        NodeConfBuilder<NoSystemId, C, CC, D, V, M>
    {
        /// Sets [`NodeConf::system_id`].
        pub fn set_system_id(self, system_id: u8) -> NodeConfBuilder<SystemId, C, CC, D, V, M> {
            NodeConfBuilder {
                system_id: SystemId(system_id),
                component_id: self.component_id,
                dialect: self.dialect,
                conn_conf: self.conn_conf,
                version: self.version,
                _marker_message: PhantomData,
            }
        }
    }

    impl<S: HasSystemId, T: HasConnConf, D: HasDialect, V: HasVersion, M: DialectMessage>
        NodeConfBuilder<S, NoComponentId, T, D, V, M>
    {
        /// Sets [`NodeConf::component_id`].
        pub fn set_component_id(
            self,
            component_id: u8,
        ) -> NodeConfBuilder<S, ComponentId, T, D, V, M> {
            NodeConfBuilder {
                system_id: self.system_id,
                component_id: ComponentId(component_id),
                dialect: self.dialect,
                conn_conf: self.conn_conf,
                version: self.version,
                _marker_message: PhantomData,
            }
        }
    }

    impl<S: HasSystemId, C: HasComponentId, D: HasDialect, V: HasVersion, M: DialectMessage>
        NodeConfBuilder<S, C, NoConnConf, D, V, M>
    {
        /// Sets [`NodeConf::component_id`].
        pub fn set_conn_conf(
            self,
            conn_conf: impl ConnectionConf + 'static,
        ) -> NodeConfBuilder<S, C, ConnConf, D, V, M> {
            NodeConfBuilder {
                system_id: self.system_id,
                component_id: self.component_id,
                dialect: self.dialect,
                conn_conf: ConnConf(Box::new(conn_conf)),
                version: self.version,
                _marker_message: PhantomData,
            }
        }
    }

    impl<S: HasSystemId, C: HasComponentId, CC: HasConnConf, V: HasVersion, M: DialectMessage>
        NodeConfBuilder<S, C, CC, NoDialect, V, M>
    {
        /// Sets [`NodeConf::dialect`].
        pub fn set_dialect<DM: DialectMessage>(
            self,
            dialect: &'static dyn DialectImpl<Message = DM>,
        ) -> NodeConfBuilder<S, C, CC, Dialect<DM>, V, DM> {
            NodeConfBuilder {
                system_id: self.system_id,
                component_id: self.component_id,
                dialect: Dialect(dialect),
                conn_conf: self.conn_conf,
                version: self.version,
                _marker_message: PhantomData,
            }
        }
    }

    impl<S: HasSystemId, C: HasComponentId, CC: HasConnConf, D: HasDialect, M: DialectMessage>
        NodeConfBuilder<S, C, CC, D, NotVersioned, M>
    {
        /// Sets [`NodeConf::dialect`].
        pub fn set_version(
            self,
            version: MavLinkVersion,
        ) -> NodeConfBuilder<S, C, CC, D, Versioned, M> {
            NodeConfBuilder {
                system_id: self.system_id,
                component_id: self.component_id,
                dialect: self.dialect,
                conn_conf: self.conn_conf,
                version: Versioned(version),
                _marker_message: PhantomData,
            }
        }
    }

    impl<M: DialectMessage, D: HasDialect, V: HasVersion>
        NodeConfBuilder<NoSystemId, NoComponentId, ConnConf, D, V, M>
    {
        /// Builds and instance of [`NodeConf`] without defined [`NodeConf::system_id`] and [`NodeConf::component_id`].
        pub fn build(self) -> NodeConf<NotIdentified, D, V, M> {
            NodeConf {
                id: NotIdentified(),
                dialect: self.dialect,
                conn_conf: self.conn_conf.0,
                version: self.version,
                _marker_message: PhantomData,
            }
        }
    }

    impl<M: DialectMessage, D: HasDialect, V: HasVersion>
        NodeConfBuilder<SystemId, ComponentId, ConnConf, D, V, M>
    {
        /// Builds and instance of [`NodeConf`] with defined [`NodeConf::system_id`] and [`NodeConf::component_id`].
        pub fn build(self) -> NodeConf<Identified, D, V, M> {
            NodeConf {
                id: Identified {
                    system_id: self.system_id.0,
                    component_id: self.component_id.0,
                },
                dialect: self.dialect,
                conn_conf: self.conn_conf.0,
                version: self.version,
                _marker_message: PhantomData,
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
            .set_system_id(10)
            .set_component_id(42)
            .set_conn_conf(TcpClientConf::new("localhost:5600").unwrap())
            .build();

        assert_eq!(node.system_id(), 10);
        assert_eq!(node.component_id(), 42);
    }

    #[test]
    fn node_conf_no_dialect_no_id_builder_workflow() {
        NodeConf::builder()
            .set_conn_conf(TcpClientConf::new("localhost:5600").unwrap())
            .build();
    }

    #[test]
    fn node_conf_builder_workflow() {
        let node = NodeConf::builder()
            .set_system_id(10)
            .set_component_id(42)
            .set_dialect(minimal::dialect())
            .set_conn_conf(TcpClientConf::new("localhost:5600").unwrap())
            .build();

        assert_eq!(node.system_id(), 10);
        assert_eq!(node.component_id(), 42);
        assert_eq!(node.dialect().name(), "minimal");
    }

    #[test]
    fn node_conf_with_dialect_encode_decode() {
        let node = NodeConf::builder()
            .set_system_id(10)
            .set_component_id(42)
            .set_dialect(minimal::dialect())
            .set_conn_conf(TcpClientConf::new("localhost:5600").unwrap())
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
