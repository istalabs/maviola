use crate::core::io::{ConnectionInfo, Retry};
use crate::core::marker::Proxy;
use crate::core::node::NodeApi;
use crate::core::utils::{Closable, SharedCloser, UniqueId};
use std::fmt::{Display, Formatter};

use crate::prelude::*;

#[derive(Clone)]
pub(crate) struct NetworkConnState {
    pub(crate) network: Closable,
    pub(crate) connection: SharedCloser,
}

#[derive(Clone, Debug)]
pub(crate) struct NetworkConnInfo {
    pub(crate) network: ConnectionInfo,
    pub(crate) connection: ConnectionInfo,
}

pub(crate) enum RestartNodeEvent<V: MaybeVersioned + 'static, A: NodeApi<V>> {
    New(UniqueId, Node<Proxy, V, A>),
    Retry(UniqueId, Retry),
    GiveUp(UniqueId),
}

impl NetworkConnState {
    pub(crate) fn is_closed(&self) -> bool {
        self.network.is_closed() || self.connection.is_closed()
    }
}

impl Display for NetworkConnInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_list()
            .entry(&self.network)
            .entry(&self.connection)
            .finish()
    }
}
