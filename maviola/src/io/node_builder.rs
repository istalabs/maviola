use std::marker::PhantomData;
use std::time::Duration;

use crate::protocol::{ComponentId, MaybeVersioned, SystemId, Versioned, Versionless};

use crate::consts::{DEFAULT_HEARTBEAT_INTERVAL, DEFAULT_HEARTBEAT_TIMEOUT};
use crate::io::marker::{
    HasComponentId, HasConnConf, HasSystemId, Identified, MaybeComponentId, MaybeConnConf,
    MaybeSystemId, NoComponentId, NoConnConf, NoSystemId, Unidentified,
};
use crate::io::NodeConf;

use crate::prelude::*;

#[cfg(feature = "sync")]
use crate::io::sync::conn::ConnectionBuilder;
#[cfg(feature = "sync")]
use crate::io::sync::marker::ConnConf;

/// Builder for [`Node`](crate::Node) and [`NodeConf`].
#[derive(Clone, Debug, Default)]
pub struct NodeBuilder<
    S: MaybeSystemId,
    C: MaybeComponentId,
    D: Dialect,
    V: MaybeVersioned,
    CC: MaybeConnConf,
> {
    pub(super) system_id: S,
    pub(super) component_id: C,
    pub(super) version: V,
    pub(super) conn_conf: CC,
    pub(super) heartbeat_timeout: Duration,
    pub(super) heartbeat_interval: Duration,
    pub(super) _dialect: PhantomData<D>,
}

impl NodeBuilder<NoSystemId, NoComponentId, Minimal, Versionless, NoConnConf> {
    /// Instantiate an empty [`NodeBuilder`].
    pub fn new() -> Self {
        Self {
            system_id: NoSystemId,
            component_id: NoComponentId,
            conn_conf: NoConnConf,
            version: Versionless,
            heartbeat_timeout: DEFAULT_HEARTBEAT_TIMEOUT,
            heartbeat_interval: DEFAULT_HEARTBEAT_INTERVAL,
            _dialect: PhantomData,
        }
    }
}

impl<C: MaybeComponentId, D: Dialect, V: MaybeVersioned, CC: MaybeConnConf>
    NodeBuilder<NoSystemId, C, D, V, CC>
{
    /// Set [`NodeConf::system_id`].
    pub fn system_id(self, system_id: SystemId) -> NodeBuilder<HasSystemId, C, D, V, CC> {
        NodeBuilder {
            system_id: HasSystemId(system_id),
            component_id: self.component_id,
            version: self.version,
            conn_conf: self.conn_conf,
            heartbeat_timeout: self.heartbeat_timeout,
            heartbeat_interval: self.heartbeat_interval,
            _dialect: self._dialect,
        }
    }
}

impl<S: MaybeSystemId, D: Dialect, V: MaybeVersioned, CC: MaybeConnConf>
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
            version: self.version,
            conn_conf: self.conn_conf,
            heartbeat_timeout: self.heartbeat_timeout,
            heartbeat_interval: self.heartbeat_interval,
            _dialect: self._dialect,
        }
    }
}

impl<S: MaybeSystemId, C: MaybeComponentId, D: Dialect, V: MaybeVersioned, CC: MaybeConnConf>
    NodeBuilder<S, C, D, V, CC>
{
    /// Set [`NodeConf::heartbeat_timeout`].
    pub fn heartbeat_timeout(self, heartbeat_timeout: Duration) -> NodeBuilder<S, C, D, V, CC> {
        NodeBuilder {
            system_id: self.system_id,
            component_id: self.component_id,
            version: self.version,
            conn_conf: self.conn_conf,
            heartbeat_timeout,
            heartbeat_interval: self.heartbeat_interval,
            _dialect: self._dialect,
        }
    }
}

#[cfg(feature = "sync")]
impl<S: MaybeSystemId, C: MaybeComponentId, D: Dialect, V: MaybeVersioned>
    NodeBuilder<S, C, D, V, NoConnConf>
{
    /// Set synchronous [`NodeConf::connection`].
    pub fn connection(
        self,
        conn_conf: impl ConnectionBuilder<V> + 'static,
    ) -> NodeBuilder<S, C, D, V, ConnConf<V>> {
        NodeBuilder {
            system_id: self.system_id,
            component_id: self.component_id,
            version: self.version,
            conn_conf: ConnConf(Box::new(conn_conf)),
            heartbeat_timeout: self.heartbeat_timeout,
            heartbeat_interval: self.heartbeat_interval,
            _dialect: self._dialect,
        }
    }
}

impl<S: MaybeSystemId, C: MaybeComponentId, D: Dialect, V: MaybeVersioned, CC: MaybeConnConf>
    NodeBuilder<S, C, D, V, CC>
{
    /// Set dialect.
    ///
    /// The dialect is a generic parameter, therefore you have to use
    /// [turbofish](https://turbo.fish/about) syntax:
    ///
    /// ```rust
    /// # use maviola::Node;
    /// use maviola::dialects::Minimal;
    ///
    /// Node::builder().dialect::<Minimal>();
    /// ```
    pub fn dialect<Dial: Dialect>(self) -> NodeBuilder<S, C, Dial, V, CC> {
        NodeBuilder {
            system_id: self.system_id,
            component_id: self.component_id,
            version: self.version,
            conn_conf: self.conn_conf,
            heartbeat_timeout: self.heartbeat_timeout,
            heartbeat_interval: self.heartbeat_interval,
            _dialect: PhantomData,
        }
    }
}

impl<S: MaybeSystemId, C: MaybeComponentId, D: Dialect, CC: MaybeConnConf>
    NodeBuilder<S, C, D, Versionless, CC>
{
    /// Set [`NodeConf::version`].
    pub fn version<Version: Versioned>(
        self,
        version: Version,
    ) -> NodeBuilder<S, C, D, Version, CC> {
        NodeBuilder {
            system_id: self.system_id,
            component_id: self.component_id,
            version,
            conn_conf: self.conn_conf,
            heartbeat_timeout: self.heartbeat_timeout,
            heartbeat_interval: self.heartbeat_interval,
            _dialect: self._dialect,
        }
    }
}

impl<V: Versioned, CC: MaybeConnConf, D: Dialect>
    NodeBuilder<HasSystemId, HasComponentId, D, V, CC>
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
    ) -> NodeBuilder<HasSystemId, HasComponentId, D, V, CC> {
        NodeBuilder {
            system_id: self.system_id,
            component_id: self.component_id,
            version: self.version,
            conn_conf: self.conn_conf,
            heartbeat_timeout: self.heartbeat_timeout,
            heartbeat_interval,
            _dialect: self._dialect,
        }
    }
}

impl<D: Dialect, V: MaybeVersioned, CC: HasConnConf>
    NodeBuilder<NoSystemId, NoComponentId, D, V, CC>
{
    /// Build and instance of [`NodeConf`] without defined [`NodeConf::system_id`] and
    /// [`NodeConf::component_id`].
    pub fn conf(self) -> NodeConf<Unidentified, D, V, CC> {
        NodeConf {
            id: Unidentified,
            version: self.version,
            connection_conf: self.conn_conf,
            heartbeat_timeout: self.heartbeat_timeout,
            heartbeat_interval: self.heartbeat_interval,
            _dialect: self._dialect,
        }
    }
}

impl<D: Dialect, V: MaybeVersioned, CC: HasConnConf>
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
            connection_conf: self.conn_conf,
            version: self.version,
            heartbeat_timeout: self.heartbeat_timeout,
            heartbeat_interval: self.heartbeat_interval,
            _dialect: self._dialect,
        }
    }
}
