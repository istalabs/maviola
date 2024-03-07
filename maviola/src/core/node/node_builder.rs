use std::marker::PhantomData;
use std::time::Duration;

use crate::protocol::{ComponentId, MessageSigner, SystemId};

use crate::core::consts::{DEFAULT_HEARTBEAT_INTERVAL, DEFAULT_HEARTBEAT_TIMEOUT};
use crate::core::marker::{
    Edge, HasComponentId, HasConnConf, HasSystemId, MaybeComponentId, MaybeConnConf, MaybeSystemId,
    NoComponentId, NoConnConf, NoSystemId, Proxy,
};
use crate::core::node::NodeConf;

use crate::prelude::*;

/// Builder for [`Node`] and [`NodeConf`].
#[derive(Clone, Debug, Default)]
pub struct NodeBuilder<
    S: MaybeSystemId,
    C: MaybeComponentId,
    D: Dialect,
    V: MaybeVersioned,
    CC: MaybeConnConf,
> {
    pub(crate) system_id: S,
    pub(crate) component_id: C,
    pub(crate) version: V,
    pub(crate) conn_conf: CC,
    pub(crate) heartbeat_timeout: Duration,
    pub(crate) heartbeat_interval: Duration,
    pub(crate) signer: Option<MessageSigner>,
    pub(crate) _dialect: PhantomData<D>,
}

impl NodeBuilder<NoSystemId, NoComponentId, Minimal, Versionless, NoConnConf> {
    /// Instantiate an empty [`NodeBuilder`].
    pub fn new() -> Self {
        Self {
            system_id: NoSystemId,
            component_id: NoComponentId,
            version: Versionless,
            conn_conf: NoConnConf,
            heartbeat_timeout: DEFAULT_HEARTBEAT_TIMEOUT,
            heartbeat_interval: DEFAULT_HEARTBEAT_INTERVAL,
            signer: None,
            _dialect: PhantomData,
        }
    }
}

impl<D: Dialect, V: MaybeVersioned, CC: MaybeConnConf>
    NodeBuilder<NoSystemId, NoComponentId, D, V, CC>
{
    /// Set [`NodeConf::system_id`] and [`NodeConf::component_id`].
    pub fn id(self, id: MavLinkId) -> NodeBuilder<HasSystemId, HasComponentId, D, V, CC> {
        NodeBuilder {
            system_id: HasSystemId(id.system),
            component_id: HasComponentId(id.component),
            version: self.version,
            conn_conf: self.conn_conf,
            heartbeat_timeout: self.heartbeat_timeout,
            heartbeat_interval: self.heartbeat_interval,
            signer: self.signer,
            _dialect: self._dialect,
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
            signer: self.signer,
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
            signer: self.signer,
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
            signer: self.signer,
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
    /// use maviola::core::node::Node;
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
            signer: self.signer,
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
            signer: self.signer,
            _dialect: self._dialect,
        }
    }
}

impl<S: MaybeSystemId, C: MaybeComponentId, D: Dialect, V: MaybeVersioned, CC: MaybeConnConf>
    NodeBuilder<S, C, D, V, CC>
{
    /// Set [`NodeConf::signer`].
    pub fn signer(self, signer: MessageSigner) -> NodeBuilder<S, C, D, V, CC> {
        NodeBuilder {
            system_id: self.system_id,
            component_id: self.component_id,
            version: self.version,
            conn_conf: self.conn_conf,
            heartbeat_timeout: self.heartbeat_timeout,
            heartbeat_interval: self.heartbeat_interval,
            signer: Some(signer),
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
            signer: self.signer,
            _dialect: self._dialect,
        }
    }
}

impl<D: Dialect, V: MaybeVersioned, CC: HasConnConf>
    NodeBuilder<NoSystemId, NoComponentId, D, V, CC>
{
    /// Build and instance of [`NodeConf`] without defined [`NodeConf::system_id`] and
    /// [`NodeConf::component_id`].
    pub fn conf(self) -> NodeConf<Proxy, D, V, CC> {
        NodeConf {
            kind: Proxy,
            version: self.version,
            connection_conf: self.conn_conf,
            heartbeat_timeout: self.heartbeat_timeout,
            heartbeat_interval: self.heartbeat_interval,
            signer: self.signer,
            _dialect: self._dialect,
        }
    }
}

impl<D: Dialect, V: MaybeVersioned, CC: HasConnConf>
    NodeBuilder<HasSystemId, HasComponentId, D, V, CC>
{
    /// Build and instance of [`NodeConf`] with defined [`NodeConf::system_id`] and
    /// [`NodeConf::component_id`].
    pub fn conf(self) -> NodeConf<Edge<V>, D, V, CC> {
        NodeConf {
            kind: Edge {
                endpoint: Endpoint::new(MavLinkId::new(self.system_id.0, self.component_id.0)),
            },
            connection_conf: self.conn_conf,
            version: self.version,
            heartbeat_timeout: self.heartbeat_timeout,
            heartbeat_interval: self.heartbeat_interval,
            signer: self.signer,
            _dialect: self._dialect,
        }
    }
}
