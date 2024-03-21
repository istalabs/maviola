use std::marker::PhantomData;

use crate::core::marker::{Edge, HasComponentId, HasSystemId, Proxy};
use crate::core::node::{NodeBuilder, NodeConf};
use crate::protocol::Unset;
use crate::sync::marker::ConnConf;

use crate::prelude::*;
use crate::sync::prelude::*;

impl<V: MaybeVersioned> NodeConf<Edge<V>, V, ConnConf<V>> {
    /// <sup>[`sync`](crate::sync)</sup>
    /// Creates a [`NodeBuilder`] initialised with current configuration.
    pub fn update(self) -> NodeBuilder<HasSystemId, HasComponentId, V, ConnConf<V>, SyncApi<V>> {
        NodeBuilder {
            system_id: HasSystemId(self.kind.endpoint.system_id()),
            component_id: HasComponentId(self.kind.endpoint.component_id()),
            conn_conf: self.connection_conf,
            heartbeat_timeout: self.heartbeat_timeout,
            heartbeat_interval: self.heartbeat_timeout,
            dialects: self.dialects,
            signer: self.signer,
            compat: self.compat,
            processors: self.processors,
            _version: PhantomData,
            _api: PhantomData,
        }
    }
}

impl<V: MaybeVersioned> NodeConf<Proxy, V, ConnConf<V>> {
    /// <sup>[`sync`](crate::sync)</sup>
    /// Creates a [`NodeBuilder`] initialised with current configuration.
    pub fn update(self) -> NodeBuilder<Unset, Unset, V, ConnConf<V>, SyncApi<V>> {
        NodeBuilder {
            system_id: Unset,
            component_id: Unset,
            conn_conf: self.connection_conf,
            heartbeat_timeout: self.heartbeat_timeout,
            heartbeat_interval: self.heartbeat_timeout,
            dialects: self.dialects,
            signer: self.signer,
            compat: self.compat,
            processors: self.processors,
            _version: PhantomData,
            _api: PhantomData,
        }
    }
}
