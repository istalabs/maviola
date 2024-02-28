//! # 🔒 Conversions for synchronous node

use crate::core::marker::{
    HasComponentId, HasSystemId, NoComponentId, NoSystemId, NodeKind, Proxy,
};
use crate::core::node::{Node, NodeBuilder, NodeConf};
use crate::sync::marker::ConnConf;
use crate::sync::node::{EdgeNode, SyncApi};

use crate::prelude::*;

impl<K: NodeKind, D: Dialect, V: MaybeVersioned + 'static> TryFrom<NodeConf<K, D, V, ConnConf<V>>>
    for Node<K, D, V, SyncApi<V>>
{
    type Error = Error;

    fn try_from(value: NodeConf<K, D, V, ConnConf<V>>) -> Result<Self> {
        Self::try_from_conf(value)
    }
}

impl<D: Dialect, V: MaybeVersioned>
    TryFrom<NodeBuilder<HasSystemId, HasComponentId, D, V, ConnConf<V>>> for EdgeNode<D, V>
{
    type Error = Error;

    fn try_from(
        value: NodeBuilder<HasSystemId, HasComponentId, D, V, ConnConf<V>>,
    ) -> Result<Self> {
        Self::try_from_conf(value.conf())
    }
}

impl<D: Dialect, V: MaybeVersioned>
    TryFrom<NodeBuilder<NoSystemId, NoComponentId, D, V, ConnConf<V>>>
    for Node<Proxy, D, V, SyncApi<V>>
{
    type Error = Error;

    fn try_from(value: NodeBuilder<NoSystemId, NoComponentId, D, V, ConnConf<V>>) -> Result<Self> {
        Self::try_from_conf(value.conf())
    }
}
