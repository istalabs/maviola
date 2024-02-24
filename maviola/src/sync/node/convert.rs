//! # Conversions for synchronous node

use crate::core::marker::{
    HasComponentId, HasSystemId, Identified, MaybeIdentified, NoComponentId, NoSystemId,
    Unidentified,
};
use crate::core::{Node, NodeBuilder, NodeConf};
use crate::sync::marker::ConnConf;
use crate::sync::node::api::SyncApi;

use crate::prelude::*;

impl<I: MaybeIdentified, D: Dialect, V: MaybeVersioned + 'static>
    TryFrom<NodeConf<I, D, V, ConnConf<V>>> for Node<I, D, V, SyncApi<V>>
{
    type Error = Error;

    fn try_from(value: NodeConf<I, D, V, ConnConf<V>>) -> Result<Self> {
        Self::try_from_conf(value)
    }
}

impl<D: Dialect, V: MaybeVersioned>
    TryFrom<NodeBuilder<HasSystemId, HasComponentId, D, V, ConnConf<V>>>
    for Node<Identified, D, V, SyncApi<V>>
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
    for Node<Unidentified, D, V, SyncApi<V>>
{
    type Error = Error;

    fn try_from(value: NodeBuilder<NoSystemId, NoComponentId, D, V, ConnConf<V>>) -> Result<Self> {
        Self::try_from_conf(value.conf())
    }
}
