use crate::core::io::{ConnectionInfo, RetryStrategy};
use crate::core::marker::Proxy;
use crate::core::node::NodeApi;
use crate::core::utils::{Closable, SharedCloser, UniqueId};
use std::fmt::{Debug, Display, Formatter};

use crate::prelude::*;

#[derive(Clone)]
pub(crate) struct NetworkConnState {
    pub(crate) network: Closable,
    pub(crate) connection: SharedCloser,
}

#[derive(Clone)]
pub(crate) struct NetworkConnInfo {
    pub(crate) network: ConnectionInfo,
    pub(crate) connection: ConnectionInfo,
}

pub(crate) enum RestartNodeEvent<V: MaybeVersioned, A: NodeApi<V>> {
    New(UniqueId, Node<Proxy, V, A>),
    Retry(UniqueId, RetryStrategy),
    GiveUp(UniqueId),
}

impl NetworkConnState {
    pub(crate) fn is_closed(&self) -> bool {
        self.network.is_closed() || self.connection.is_closed()
    }
}

impl Debug for NetworkConnInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NetworkConnInfo")
            .field("connection", &self.connection)
            .finish_non_exhaustive()
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
