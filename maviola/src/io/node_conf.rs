//! MAVLink node configuration.

use std::marker::PhantomData;
use std::sync::Arc;

use mavio::protocol::{DialectImpl, DialectMessage};

use crate::io::sync::ConnectionConf;

/// MAVLink node configuration.
///
/// Node configuration can be instantiated only through [`NodeConfBuilder`](builder::NodeConfBuilder).
pub struct NodeConf<I: variants::HasId, D: variants::HasDialect, M: DialectMessage + 'static> {
    system_id: Option<u8>,
    component_id: Option<u8>,
    dialect: Option<&'static dyn DialectImpl<Message = M>>,
    conn_conf: Option<Arc<dyn ConnectionConf>>,
    _has_dialect: PhantomData<D>,
    _has_id: PhantomData<I>,
}

/// Variants of [`NodeConf`].
pub mod variants {
    /// Marker for [`NodeConf`](super::NodeConf) with or without [`dialect`](super::NodeConf::dialect).
    ///
    /// Variants:
    ///
    /// * [`NoDialect`]
    /// * [`WithDialect`]
    pub trait HasDialect {}

    /// Variant of [`NodeConf`](super::NodeConf) without a [`dialect`](super::NodeConf::dialect).
    /// Message decoding is not possible, only raw MAVLink frames can be communicated.
    pub struct NoDialect();
    impl HasDialect for NoDialect {}

    /// Variant of [`NodeConf`](super::NodeConf) with a [`dialect`](super::NodeConf::dialect) being
    /// specified. Message encoding/decoding is available.
    pub struct WithDialect();
    impl HasDialect for WithDialect {}

    /// Marker for [`NodeConf`](super::NodeConf) with or without
    /// [`system_id`](super::NodeConf::system_id) and [`component_id`](super::NodeConf::component_id).
    ///
    /// Variants:
    ///
    /// * [`NoId`]
    /// * [`WithId`]
    pub trait HasId {}

    /// Variant of [`NodeConf`](super::NodeConf) without [`system_id`](super::NodeConf::system_id)
    /// and [`component_id`](super::NodeConf::component_id). This node can't produce messages and
    /// can be used only as a proxy.
    pub struct NoId();
    impl HasId for NoId {}

    /// Variant of [`NodeConf`](super::NodeConf) with [`system_id`](super::NodeConf::system_id)
    /// and [`component_id`](super::NodeConf::component_id) being defined. This node can produce
    /// messages.
    pub struct WithId();
    impl HasId for WithId {}
}

/// Builder for [`NodeConf`].
pub mod builder {
    use crate::io::sync::ConnectionConf;
    use mavio::protocol::{DialectImpl, DialectMessage};
    use std::marker::PhantomData;
    use std::sync::Arc;

    use super::variants::{HasDialect, NoDialect, NoId, WithDialect, WithId};
    use super::NodeConf;

    /// Marker trait for [`NodeConfBuilder`] with or without [`NodeConf::system_id`].
    pub trait HasSystemId {}

    /// Marker for [`NodeConfBuilder`] without [`NodeConf::system_id`].
    pub struct NoSystemId();
    impl HasSystemId for NoSystemId {}

    /// Marker for [`NodeConfBuilder`] with [`NodeConf::system_id`] set.
    pub struct WithSystemId();
    impl HasSystemId for WithSystemId {}

    /// Marker trait for [`NodeConfBuilder`] with or without [`NodeConf::component_id`].
    pub trait HasComponentId {}

    /// Marker for [`NodeConfBuilder`] without [`NodeConf::component_id`].
    pub struct NoComponentId();
    impl HasComponentId for NoComponentId {}

    /// Marker for [`NodeConfBuilder`] with [`NodeConf::component_id`] set.
    pub struct WithComponentId();
    impl HasComponentId for WithComponentId {}

    /// Marker trait for [`NodeConfBuilder`] with or without [`NodeConf::conn_conf`].
    pub trait HasConnectionConf {}

    /// Marker for [`NodeConfBuilder`] without [`NodeConf::conn_conf`].
    pub struct NoConnectionConf();
    impl HasConnectionConf for NoConnectionConf {}

    /// Marker for [`NodeConfBuilder`] with [`NodeConf::conn_conf`] set.
    pub struct WithConnectionConf();
    impl HasConnectionConf for WithConnectionConf {}

    /// Builder for [`NodeConf`].
    #[derive(Clone, Default)]
    pub struct NodeConfBuilder<
        S: HasSystemId,
        C: HasComponentId,
        T: HasConnectionConf,
        D: HasDialect,
        M: DialectMessage + 'static,
    > {
        system_id: Option<u8>,
        component_id: Option<u8>,
        dialect: Option<&'static dyn DialectImpl<Message = M>>,
        conn_conf: Option<Arc<dyn ConnectionConf>>,
        _has_system_id: PhantomData<S>,
        _has_component_id: PhantomData<C>,
        _has_conn_conf: PhantomData<T>,
        _has_dialect: PhantomData<D>,
    }

    impl<M: DialectMessage> NodeConfBuilder<NoSystemId, NoComponentId, NoConnectionConf, NoDialect, M> {
        /// Instantiates an empty [`NodeConfBuilder`].
        pub fn new() -> Self {
            Self {
                system_id: None,
                component_id: None,
                dialect: None,
                conn_conf: None,
                _has_system_id: Default::default(),
                _has_component_id: Default::default(),
                _has_conn_conf: Default::default(),
                _has_dialect: Default::default(),
            }
        }
    }

    impl<C: HasComponentId, T: HasConnectionConf, D: HasDialect, M: DialectMessage>
        NodeConfBuilder<NoSystemId, C, T, D, M>
    {
        /// Sets [`NodeConf::system_id`].
        pub fn set_system_id(&self, system_id: u8) -> NodeConfBuilder<WithSystemId, C, T, D, M> {
            NodeConfBuilder {
                system_id: Some(system_id),
                component_id: self.component_id,
                dialect: self.dialect,
                conn_conf: self.conn_conf.clone(),
                _has_system_id: PhantomData,
                _has_component_id: PhantomData,
                _has_conn_conf: PhantomData,
                _has_dialect: PhantomData,
            }
        }
    }

    impl<S: HasSystemId, T: HasConnectionConf, D: HasDialect, M: DialectMessage>
        NodeConfBuilder<S, NoComponentId, T, D, M>
    {
        /// Sets [`NodeConf::component_id`].
        pub fn set_component_id(
            &self,
            component_id: u8,
        ) -> NodeConfBuilder<S, WithComponentId, T, D, M> {
            NodeConfBuilder {
                system_id: self.system_id,
                component_id: Some(component_id),
                dialect: self.dialect,
                conn_conf: self.conn_conf.clone(),
                _has_system_id: PhantomData,
                _has_component_id: PhantomData,
                _has_conn_conf: PhantomData,
                _has_dialect: PhantomData,
            }
        }
    }

    impl<S: HasSystemId, C: HasComponentId, D: HasDialect, M: DialectMessage>
        NodeConfBuilder<S, C, NoConnectionConf, D, M>
    {
        /// Sets [`NodeConf::component_id`].
        pub fn set_conn_conf(
            &self,
            conn_conf: impl ConnectionConf + 'static,
        ) -> NodeConfBuilder<S, C, WithConnectionConf, D, M> {
            NodeConfBuilder {
                system_id: self.system_id,
                component_id: self.component_id,
                dialect: self.dialect,
                conn_conf: Some(Arc::new(conn_conf)),
                _has_system_id: PhantomData,
                _has_component_id: PhantomData,
                _has_conn_conf: PhantomData,
                _has_dialect: PhantomData,
            }
        }
    }

    impl<S: HasSystemId, C: HasComponentId, T: HasConnectionConf, M: DialectMessage>
        NodeConfBuilder<S, C, T, NoDialect, M>
    {
        /// Sets [`NodeConf::dialect`].
        pub fn set_dialect<DM: DialectMessage>(
            &self,
            dialect: &'static dyn DialectImpl<Message = DM>,
        ) -> NodeConfBuilder<S, C, T, WithDialect, DM> {
            NodeConfBuilder {
                system_id: self.system_id,
                component_id: self.component_id,
                dialect: Some(dialect),
                conn_conf: self.conn_conf.clone(),
                _has_system_id: PhantomData,
                _has_component_id: PhantomData,
                _has_conn_conf: PhantomData,
                _has_dialect: PhantomData,
            }
        }
    }

    impl<M: DialectMessage>
        NodeConfBuilder<NoSystemId, NoComponentId, WithConnectionConf, NoDialect, M>
    {
        /// Builds and instance of [`NodeConf`] without defined [`NodeConf::system_id`],
        /// [`NodeConf::component_id`], and [`NodeConf::dialect`].
        pub fn build(&self) -> NodeConf<NoId, NoDialect, crate::dialects::minimal::Message> {
            NodeConf {
                system_id: None,
                component_id: None,
                dialect: None,
                conn_conf: self.conn_conf.clone(),
                _has_dialect: PhantomData,
                _has_id: PhantomData,
            }
        }
    }

    impl<M: DialectMessage>
        NodeConfBuilder<WithSystemId, WithComponentId, WithConnectionConf, NoDialect, M>
    {
        /// Builds and instance of [`NodeConf`] with defined [`NodeConf::system_id`] and
        /// [`NodeConf::component_id`] without a specific [`NodeConf::dialect`].
        pub fn build(&self) -> NodeConf<WithId, NoDialect, crate::dialects::minimal::Message> {
            NodeConf {
                system_id: self.system_id,
                component_id: self.component_id,
                dialect: None,
                conn_conf: self.conn_conf.clone(),
                _has_dialect: PhantomData,
                _has_id: PhantomData,
            }
        }
    }

    impl<M: DialectMessage>
        NodeConfBuilder<NoSystemId, NoComponentId, WithConnectionConf, WithDialect, M>
    {
        /// Builds and instance of [`NodeConf`] without defined [`NodeConf::system_id`],
        /// [`NodeConf::component_id`] and specified [`NodeConf::dialect`].
        pub fn build(&self) -> NodeConf<NoId, WithDialect, M> {
            NodeConf {
                system_id: None,
                component_id: None,
                dialect: self.dialect,
                conn_conf: self.conn_conf.clone(),
                _has_dialect: PhantomData,
                _has_id: PhantomData,
            }
        }
    }

    impl<M: DialectMessage>
        NodeConfBuilder<WithSystemId, WithComponentId, WithConnectionConf, WithDialect, M>
    {
        /// Builds and instance of [`NodeConf`] with defined [`NodeConf::system_id`],
        /// [`NodeConf::component_id`], and [`NodeConf::dialect`].
        pub fn build(&self) -> NodeConf<WithId, WithDialect, M> {
            NodeConf {
                system_id: self.system_id,
                component_id: self.component_id,
                dialect: self.dialect,
                conn_conf: self.conn_conf.clone(),
                _has_dialect: PhantomData,
                _has_id: PhantomData,
            }
        }
    }
}

impl<D: variants::HasDialect, M: DialectMessage> NodeConf<variants::WithId, D, M> {
    /// MAVLink system ID.
    pub fn system_id(&self) -> u8 {
        self.system_id.unwrap()
    }

    /// MAVLink component ID.
    pub fn component_id(&self) -> u8 {
        self.component_id.unwrap()
    }
}

impl NodeConf<variants::NoId, variants::NoDialect, crate::dialects::minimal::Message> {
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
        builder::NoConnectionConf,
        variants::NoDialect,
        crate::dialects::minimal::Message,
    > {
        builder::NodeConfBuilder::new()
    }
}

impl<I: variants::HasId, M: DialectMessage> NodeConf<I, variants::WithDialect, M> {
    /// MAVLink dialect.
    pub fn dialect(&self) -> &'static dyn DialectImpl<Message = M> {
        self.dialect.unwrap()
    }
}

impl<I: variants::HasId, D: variants::HasDialect, M: DialectMessage> NodeConf<I, D, M> {
    /// Connection configuration.
    pub fn conn_conf(&self) -> &dyn ConnectionConf {
        self.conn_conf.as_ref().unwrap().as_ref()
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
