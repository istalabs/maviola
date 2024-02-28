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
///
/// An edge node is a MAVlink device with defined system `ID` and component `ID`. It can send and
/// receive MAVLink messages, emit automatic heartbeats and perform other active tasks.
///
/// # Examples
///
/// Create a synchronous TCP server node:
///
/// ```no_run
/// use maviola::prelude::*;
/// use maviola::sync::prelude::*;
///
/// let addr = "127.0.0.1:5600";
///
/// // Create a node from configuration
/// let mut node = Node::builder()
///     .version(V2)                // restrict node to MAVLink2 protocol version
///     .system_id(1)               // System `ID`
///     .component_id(1)            // Component `ID`
///     .dialect::<Minimal>()       // Dialect is set to `minimal`
///     .connection(
///         TcpServer::new(addr)    // Configure TCP server connection
///             .unwrap()
///     ).build().unwrap();
///
/// // Activate node to start sending heartbeats
/// node.activate().unwrap();
///
/// // Process incoming events
/// for event in node.events() {
///     match event {
///         Event::NewPeer(peer) => {
///             /* handle a new peer */
///         }
///         Event::PeerLost(peer) => {
///             /* handle a peer, that becomes inactive */
///         }
///         Event::Frame(frame, res) => {
///             // Send back any incoming frame directly to its sender's channel
///             res.respond(&frame).unwrap();
///         }
///     }
/// }
/// ```
pub type EdgeNode<D, V> = Node<Edge<V>, D, V, SyncApi<V>>;

/// <sup>[`sync`](crate::sync)</sup>
/// Synchronous node representing a MAVLink proxy.
pub type ProxyNode<D, V> = Node<Proxy, D, V, SyncApi<V>>;
