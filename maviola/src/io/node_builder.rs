use std::time::Duration;

use mavio::protocol::{
    ComponentId, DialectImpl, DialectMessage, MaybeVersioned, SystemId, Versioned, Versionless,
};

use crate::consts::{DEFAULT_HEARTBEAT_INTERVAL, DEFAULT_HEARTBEAT_TIMEOUT};
use crate::io::marker::{HasConnConf, Identified, MaybeConnConf, NoConnConf, Unidentified};
use crate::io::NodeConf;
use crate::protocol::{Dialectless, HasDialect, MaybeDialect};

#[cfg(feature = "sync")]
use crate::io::sync::connection::ConnectionConf;
#[cfg(feature = "sync")]
use crate::io::sync::marker::SyncConnConf;

/// Marker trait for [`NodeBuilder`] with or without [`NodeConf::system_id`].
pub trait MaybeSystemId {}

/// Marker for [`NodeBuilder`] without [`NodeConf::system_id`].
pub struct NoSystemId;
impl MaybeSystemId for NoSystemId {}

/// Marker for [`NodeBuilder`] with [`NodeConf::system_id`] set.
pub struct HasSystemId(pub(super) SystemId);
impl MaybeSystemId for HasSystemId {}

/// Marker trait for [`NodeBuilder`] with or without [`NodeConf::component_id`].
pub trait MaybeComponentId {}

/// Marker for [`NodeBuilder`] without [`NodeConf::component_id`].
pub struct NoComponentId;
impl MaybeComponentId for NoComponentId {}

/// Marker for [`NodeBuilder`] with [`NodeConf::component_id`] set.
pub struct HasComponentId(pub(super) ComponentId);
impl MaybeComponentId for HasComponentId {}

/// Builder for [`Node`](crate::Node) and [`NodeConf`].
#[derive(Clone, Debug, Default)]
pub struct NodeBuilder<
    S: MaybeSystemId,
    C: MaybeComponentId,
    D: MaybeDialect,
    V: MaybeVersioned,
    CC: MaybeConnConf,
> {
    pub(super) system_id: S,
    pub(super) component_id: C,
    pub(super) dialect: D,
    pub(super) version: V,
    pub(super) conn_conf: CC,
    pub(super) heartbeat_timeout: Duration,
    pub(super) heartbeat_interval: Duration,
}

impl NodeBuilder<NoSystemId, NoComponentId, Dialectless, Versionless, NoConnConf> {
    /// Instantiate an empty [`NodeBuilder`].
    pub fn new() -> Self {
        Self {
            system_id: NoSystemId,
            component_id: NoComponentId,
            dialect: Dialectless,
            conn_conf: NoConnConf,
            version: Versionless,
            heartbeat_timeout: DEFAULT_HEARTBEAT_TIMEOUT,
            heartbeat_interval: DEFAULT_HEARTBEAT_INTERVAL,
        }
    }
}

impl<C: MaybeComponentId, D: MaybeDialect, V: MaybeVersioned, CC: MaybeConnConf>
    NodeBuilder<NoSystemId, C, D, V, CC>
{
    /// Set [`NodeConf::system_id`].
    pub fn system_id(self, system_id: SystemId) -> NodeBuilder<HasSystemId, C, D, V, CC> {
        NodeBuilder {
            system_id: HasSystemId(system_id),
            component_id: self.component_id,
            dialect: self.dialect,
            version: self.version,
            conn_conf: self.conn_conf,
            heartbeat_timeout: self.heartbeat_timeout,
            heartbeat_interval: self.heartbeat_interval,
        }
    }
}

impl<S: MaybeSystemId, D: MaybeDialect, V: MaybeVersioned, CC: MaybeConnConf>
    NodeBuilder<S, NoComponentId, D, V, CC>
{
    /// Set [`NodeConf::component_id`].
    pub fn component_id(
        self,
        component_id: ComponentId,
    ) -> NodeBuilder<S, HasComponentId, D, V, CC> {
        NodeBuilder {
            system_id: self.system_id,
            component_id: HasComponentId(component_id),
            dialect: self.dialect,
            version: self.version,
            conn_conf: self.conn_conf,
            heartbeat_timeout: self.heartbeat_timeout,
            heartbeat_interval: self.heartbeat_interval,
        }
    }
}

impl<
        S: MaybeSystemId,
        C: MaybeComponentId,
        D: MaybeDialect,
        V: MaybeVersioned,
        CC: MaybeConnConf,
    > NodeBuilder<S, C, D, V, CC>
{
    /// Set [`NodeConf::heartbeat_timeout`].
    pub fn heartbeat_timeout(self, heartbeat_timeout: Duration) -> NodeBuilder<S, C, D, V, CC> {
        NodeBuilder {
            system_id: self.system_id,
            component_id: self.component_id,
            dialect: self.dialect,
            version: self.version,
            conn_conf: self.conn_conf,
            heartbeat_timeout,
            heartbeat_interval: self.heartbeat_interval,
        }
    }
}

#[cfg(feature = "sync")]
impl<S: MaybeSystemId, C: MaybeComponentId, D: MaybeDialect, V: MaybeVersioned>
    NodeBuilder<S, C, D, V, NoConnConf>
{
    /// Set synchronous [`NodeConf::connection`].
    pub fn connection(
        self,
        conn_conf: impl ConnectionConf<V> + 'static,
    ) -> NodeBuilder<S, C, D, V, SyncConnConf<V>> {
        NodeBuilder {
            system_id: self.system_id,
            component_id: self.component_id,
            dialect: self.dialect,
            version: self.version,
            conn_conf: SyncConnConf(Box::new(conn_conf)),
            heartbeat_timeout: self.heartbeat_timeout,
            heartbeat_interval: self.heartbeat_interval,
        }
    }
}

impl<S: MaybeSystemId, C: MaybeComponentId, V: MaybeVersioned, CC: MaybeConnConf>
    NodeBuilder<S, C, Dialectless, V, CC>
{
    /// Set [`NodeConf::dialect`].
    pub fn dialect<M: DialectMessage>(
        self,
        dialect: &'static dyn DialectImpl<Message = M>,
    ) -> NodeBuilder<S, C, HasDialect<M>, V, CC> {
        NodeBuilder {
            system_id: self.system_id,
            component_id: self.component_id,
            dialect: HasDialect(dialect),
            version: self.version,
            conn_conf: self.conn_conf,
            heartbeat_timeout: self.heartbeat_timeout,
            heartbeat_interval: self.heartbeat_interval,
        }
    }
}

impl<S: MaybeSystemId, C: MaybeComponentId, D: MaybeDialect, CC: MaybeConnConf>
    NodeBuilder<S, C, D, Versionless, CC>
{
    /// Set [`NodeConf::dialect`].
    pub fn version<Version: Versioned>(
        self,
        version: Version,
    ) -> NodeBuilder<S, C, D, Version, CC> {
        NodeBuilder {
            system_id: self.system_id,
            component_id: self.component_id,
            dialect: self.dialect,
            version,
            conn_conf: self.conn_conf,
            heartbeat_timeout: self.heartbeat_timeout,
            heartbeat_interval: self.heartbeat_interval,
        }
    }
}

impl<V: Versioned, CC: MaybeConnConf, M: DialectMessage>
    NodeBuilder<HasSystemId, HasComponentId, HasDialect<M>, V, CC>
{
    /// Set [`NodeConf::heartbeat_interval`].
    ///
    /// This parameter makes sense only for nodes that are identified, has a specified dialect
    /// and MAVLink protocol version. Therefore, the method is available only when the following
    /// parameters have been already set:
    ///
    /// * [`system_id`](NodeBuilder::system_id)
    /// * [`component_id`](NodeBuilder::component_id)
    /// * [`dialect`](NodeBuilder::dialect)
    /// * [`version`](NodeBuilder::version)
    pub fn heartbeat_interval(
        self,
        heartbeat_interval: Duration,
    ) -> NodeBuilder<HasSystemId, HasComponentId, HasDialect<M>, V, CC> {
        NodeBuilder {
            system_id: self.system_id,
            component_id: self.component_id,
            dialect: self.dialect,
            version: self.version,
            conn_conf: self.conn_conf,
            heartbeat_timeout: self.heartbeat_timeout,
            heartbeat_interval,
        }
    }
}

impl<D: MaybeDialect, V: MaybeVersioned, CC: HasConnConf>
    NodeBuilder<NoSystemId, NoComponentId, D, V, CC>
{
    /// Build and instance of [`NodeConf`] without defined [`NodeConf::system_id`] and
    /// [`NodeConf::component_id`].
    pub fn conf(self) -> NodeConf<Unidentified, D, V, CC> {
        NodeConf {
            id: Unidentified,
            dialect: self.dialect,
            version: self.version,
            connection_conf: self.conn_conf,
            heartbeat_timeout: self.heartbeat_timeout,
            heartbeat_interval: self.heartbeat_interval,
        }
    }
}

impl<D: MaybeDialect, V: MaybeVersioned, CC: HasConnConf>
    NodeBuilder<HasSystemId, HasComponentId, D, V, CC>
{
    /// Build and instance of [`NodeConf`] with defined [`NodeConf::system_id`] and
    /// [`NodeConf::component_id`].
    pub fn conf(self) -> NodeConf<Identified, D, V, CC> {
        NodeConf {
            id: Identified {
                system_id: self.system_id.0,
                component_id: self.component_id.0,
            },
            dialect: self.dialect,
            connection_conf: self.conn_conf,
            version: self.version,
            heartbeat_timeout: self.heartbeat_timeout,
            heartbeat_interval: self.heartbeat_interval,
        }
    }
}
