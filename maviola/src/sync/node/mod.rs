//! # API extensions for synchronous MAVLink node

mod api;
mod build_ext;
mod convert;
mod event;
mod ext;
mod handler;

pub use api::SyncApi;
pub use event::Event;

use crate::core::marker::{Edge, Proxy};
use crate::core::node::Node;

/// <sup>[`sync`](crate::sync)</sup>
/// Synchronous node representing an edge MAVLink device.
pub type EdgeNode<D, V> = Node<Edge<V>, D, V, SyncApi<V>>;

/// <sup>[`sync`](crate::sync)</sup>
/// Synchronous node representing a MAVLink proxy.
pub type ProxyNode<D, V> = Node<Proxy, D, V, SyncApi<V>>;
