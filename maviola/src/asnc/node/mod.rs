//! # API extensions for asynchronous MAVLink node

mod api;
mod build_ext;
mod event;
mod ext;
pub(super) mod handler;

pub use api::AsyncApi;
pub use event::Event;

use crate::core::marker::{Edge, Proxy};
use crate::core::node::Node;

/// <sup>[`async`](crate::asnc)</sup>
/// Asynchronous node representing an edge MAVLink device.
///
/// An edge node is a MAVlink device with defined system `ID` and component `ID`. It can send and
/// receive MAVLink messages, emit automatic heartbeats and perform other active tasks.
///
/// # Examples
///
/// Create an asynchronous TCP server node:
///
/// ```rust,no_run
/// # #[tokio::main(flavor = "current_thread")] async fn main() {
/// use tokio_stream::StreamExt;
/// use maviola::prelude::*;
/// use maviola::asnc::prelude::*;
///
/// let addr = "127.0.0.1:5600";
///
/// // Create a node from configuration
/// let mut node = Node::builder()
///     .version(V2)                // restrict node to MAVLink2 protocol version
///     .system_id(1)               // System `ID`
///     .component_id(1)            // Component `ID`
///     .dialect::<Minimal>()       // Dialect is set to `minimal`
///     .async_connection(
///         TcpServer::new(addr)    // Configure TCP server connection
///             .unwrap()
///     ).build().await.unwrap();
///
/// // Activate node to start sending heartbeats
/// node.activate().await.unwrap();
///
/// // Process incoming events
/// let mut events = node.events().unwrap();
/// while let Some(event) = events.next().await {
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
///         Event::Invalid(frame, err, callback) => {
///             /* Process invalid frame */
///         }
///     }
/// }
/// # }
/// ```
pub type EdgeNode<D, V> = Node<Edge<V>, D, V, AsyncApi<V>>;

/// <sup>[`async`](crate::asnc)</sup>
/// Asynchronous node representing a MAVLink proxy.
pub type ProxyNode<D, V> = Node<Proxy, D, V, AsyncApi<V>>;
