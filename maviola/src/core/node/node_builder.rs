use std::marker::PhantomData;
use std::time::Duration;

use crate::core::consts::{DEFAULT_HEARTBEAT_INTERVAL, DEFAULT_HEARTBEAT_TIMEOUT};
use crate::core::marker::{
    Edge, HasComponentId, HasConnConf, HasSystemId, MaybeComponentId, MaybeConnConf, MaybeSystemId,
    Proxy, Unset,
};
use crate::core::node::node_conf::IntoNodeConf;
use crate::core::node::{NodeApi, NodeConf};
#[cfg(feature = "unsafe")]
use crate::protocol::ProcessFrame;
use crate::protocol::{
    ComponentId, CustomFrameProcessors, IntoCompatProcessor, IntoFrameSigner, KnownDialects,
    SystemId,
};

use crate::prelude::*;

/// Builder for [`Node`] and [`NodeConf`].
#[derive(Clone, Debug)]
pub struct NodeBuilder<
    S: MaybeSystemId,
    C: MaybeComponentId,
    V: MaybeVersioned,
    CC: MaybeConnConf,
    A: NodeApi<V>,
> {
    pub(crate) system_id: S,
    pub(crate) component_id: C,
    pub(crate) conn_conf: CC,
    pub(crate) heartbeat_timeout: Duration,
    pub(crate) heartbeat_interval: Duration,
    pub(crate) dialects: KnownDialects,
    pub(crate) signer: Option<FrameSigner>,
    pub(crate) compat: Option<CompatProcessor>,
    pub(crate) processors: CustomFrameProcessors,
    pub(crate) _version: PhantomData<V>,
    pub(crate) _api: PhantomData<A>,
}

impl NodeBuilder<Unset, Unset, Versionless, Unset, Unset> {
    /// Instantiate an empty versionless [`NodeBuilder`].
    pub fn new() -> Self {
        Self {
            system_id: Unset,
            component_id: Unset,
            conn_conf: Unset,
            heartbeat_timeout: DEFAULT_HEARTBEAT_TIMEOUT,
            heartbeat_interval: DEFAULT_HEARTBEAT_INTERVAL,
            dialects: Default::default(),
            signer: None,
            compat: None,
            processors: Default::default(),
            _version: PhantomData,
            _api: PhantomData,
        }
    }
}

impl<S: MaybeSystemId, C: MaybeComponentId, V: MaybeVersioned, CC: MaybeConnConf>
    NodeBuilder<S, C, V, CC, Unset>
{
    /// Set [`NodeConf::version`].
    pub fn version<Version: MaybeVersioned>(self) -> NodeBuilder<S, C, Version, CC, Unset> {
        NodeBuilder {
            system_id: self.system_id,
            component_id: self.component_id,
            conn_conf: self.conn_conf,
            heartbeat_timeout: self.heartbeat_timeout,
            heartbeat_interval: self.heartbeat_interval,
            dialects: self.dialects,
            signer: self.signer,
            compat: self.compat,
            processors: self.processors,
            _version: PhantomData,
            _api: PhantomData,
        }
    }
}

impl<V: MaybeVersioned, CC: MaybeConnConf, A: NodeApi<V>> NodeBuilder<Unset, Unset, V, CC, A> {
    /// Set [`NodeConf::system_id`] and [`NodeConf::component_id`].
    pub fn id(self, id: MavLinkId) -> NodeBuilder<HasSystemId, HasComponentId, V, CC, A> {
        NodeBuilder {
            system_id: HasSystemId(id.system),
            component_id: HasComponentId(id.component),
            conn_conf: self.conn_conf,
            heartbeat_timeout: self.heartbeat_timeout,
            heartbeat_interval: self.heartbeat_interval,
            dialects: self.dialects,
            signer: self.signer,
            compat: self.compat,
            processors: self.processors,
            _version: self._version,
            _api: self._api,
        }
    }
}

impl<C: MaybeComponentId, V: MaybeVersioned, CC: MaybeConnConf, A: NodeApi<V>>
    NodeBuilder<Unset, C, V, CC, A>
{
    /// Set [`NodeConf::system_id`].
    pub fn system_id(self, system_id: SystemId) -> NodeBuilder<HasSystemId, C, V, CC, A> {
        NodeBuilder {
            system_id: HasSystemId(system_id),
            component_id: self.component_id,
            conn_conf: self.conn_conf,
            heartbeat_timeout: self.heartbeat_timeout,
            heartbeat_interval: self.heartbeat_interval,
            dialects: self.dialects,
            signer: self.signer,
            compat: self.compat,
            processors: self.processors,
            _version: self._version,
            _api: self._api,
        }
    }
}

impl<S: MaybeSystemId, V: MaybeVersioned, CC: MaybeConnConf, A: NodeApi<V>>
    NodeBuilder<S, Unset, V, CC, A>
{
    /// Set [`NodeConf::component_id`].
    pub fn component_id(
        self,
        component_id: ComponentId,
    ) -> NodeBuilder<S, HasComponentId, V, CC, A> {
        NodeBuilder {
            system_id: self.system_id,
            component_id: HasComponentId(component_id),
            conn_conf: self.conn_conf,
            heartbeat_timeout: self.heartbeat_timeout,
            heartbeat_interval: self.heartbeat_interval,
            dialects: self.dialects,
            signer: self.signer,
            compat: self.compat,
            processors: self.processors,
            _version: self._version,
            _api: self._api,
        }
    }
}

impl<
        S: MaybeSystemId,
        C: MaybeComponentId,
        V: MaybeVersioned,
        CC: MaybeConnConf,
        A: NodeApi<V>,
    > NodeBuilder<S, C, V, CC, A>
{
    /// Set [`NodeConf::heartbeat_timeout`].
    pub fn heartbeat_timeout(self, heartbeat_timeout: Duration) -> Self {
        NodeBuilder {
            heartbeat_timeout,
            ..self
        }
    }

    /// Set [`NodeConf::signer`].
    ///
    /// Accepts anything, that implements [`IntoFrameSigner`].
    pub fn signer(self, signer: impl IntoFrameSigner) -> Self {
        NodeBuilder {
            signer: Some(signer.into_message_signer()),
            ..self
        }
    }

    /// Set [`NodeConf::compat`].
    pub fn compat(self, compat: impl IntoCompatProcessor) -> Self {
        NodeBuilder {
            compat: Some(compat.into_compat_processor()),
            ..self
        }
    }

    /// Set main [`NodeConf::dialect`].
    ///
    /// Dialect should be specified via [turbofish](https://turbo.fish/about) syntax.
    ///
    /// Default dialect is `minimal`.
    pub fn dialect<D: Dialect>(mut self) -> Self {
        self.dialects = self.dialects.with_dialect(D::spec());
        self
    }

    /// Adds dialect to [`NodeConf::known_dialects`].
    ///
    /// Node can perform frame validation against known dialects. However, automatic operations,
    /// like heartbeats, will use the main [`NodeConf::dialect`].
    ///
    /// Dialect should be specified via [turbofish](https://turbo.fish/about) syntax.
    ///
    /// Default `minimal` is always among the known dialects. Internally, dialect names are used as
    /// a dialect `ID`. So, it is technically possible to replace default dialect, but we strongly
    /// advice against doing that.
    pub fn add_dialect<D: Dialect>(mut self) -> Self {
        self.dialects = self.dialects.with_known_dialect(D::spec());
        self
    }

    /// Adds a custom frame processor, that implements [`ProcessFrame`].
    #[cfg(feature = "unsafe")]
    pub fn add_processor(
        mut self,
        name: &'static str,
        processor: impl ProcessFrame + 'static,
    ) -> Self {
        self.processors.add(name, processor);
        self
    }
}

impl<V: Versioned, CC: MaybeConnConf, A: NodeApi<V>>
    NodeBuilder<HasSystemId, HasComponentId, V, CC, A>
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
    ) -> NodeBuilder<HasSystemId, HasComponentId, V, CC, A> {
        NodeBuilder {
            heartbeat_interval,
            ..self
        }
    }
}

impl<V: MaybeVersioned, CC: HasConnConf, A: NodeApi<V>> NodeBuilder<Unset, Unset, V, CC, A> {
    /// Build and instance of [`NodeConf`] without defined [`NodeConf::system_id`] and
    /// [`NodeConf::component_id`].
    pub fn conf(self) -> NodeConf<Proxy, V, CC> {
        NodeConf {
            kind: Proxy,
            connection_conf: self.conn_conf,
            heartbeat_timeout: self.heartbeat_timeout,
            heartbeat_interval: self.heartbeat_interval,
            dialects: self.dialects,
            signer: self.signer,
            compat: self.compat,
            processors: self.processors,
            _version: PhantomData,
        }
    }
}

impl<V: MaybeVersioned, CC: HasConnConf, A: NodeApi<V>>
    NodeBuilder<HasSystemId, HasComponentId, V, CC, A>
{
    /// Build and instance of [`NodeConf`] with defined [`NodeConf::system_id`] and
    /// [`NodeConf::component_id`].
    pub fn conf(self) -> NodeConf<Edge<V>, V, CC> {
        NodeConf {
            kind: Edge {
                endpoint: Endpoint::new(MavLinkId::new(self.system_id.0, self.component_id.0)),
            },
            connection_conf: self.conn_conf,
            heartbeat_timeout: self.heartbeat_timeout,
            heartbeat_interval: self.heartbeat_interval,
            dialects: self.dialects,
            signer: self.signer,
            compat: self.compat,
            processors: self.processors,
            _version: PhantomData,
        }
    }
}

impl Default for NodeBuilder<Unset, Unset, Versionless, Unset, Unset> {
    /// Instantiate an empty versionless [`NodeBuilder`].
    fn default() -> Self {
        Self::new()
    }
}

impl<V: MaybeVersioned, C: HasConnConf, A: NodeApi<V>> IntoNodeConf<Proxy, V, C>
    for NodeBuilder<Unset, Unset, V, C, A>
{
    fn into_node_conf(self) -> NodeConf<Proxy, V, C> {
        self.conf()
    }
}

impl<V: MaybeVersioned, C: HasConnConf, A: NodeApi<V>> IntoNodeConf<Edge<V>, V, C>
    for NodeBuilder<HasSystemId, HasComponentId, V, C, A>
{
    fn into_node_conf(self) -> NodeConf<Edge<V>, V, C> {
        self.conf()
    }
}
