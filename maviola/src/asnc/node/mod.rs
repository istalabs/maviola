//! # API extensions for asynchronous MAVLink node

pub(in crate::asnc) mod api;
mod build_ext;
mod callback;
mod conf_ext;
mod event;
mod ext;
pub(super) mod handler;
mod receive;
mod receiver;
mod sender;

pub use api::AsyncApi;
pub use callback::Callback;
pub use event::Event;
pub use receive::{ReceiveEvent, ReceiveFrame};
pub use receiver::EventReceiver;
pub use sender::FrameSender;

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
/// let mut node = Node::asnc::<V2>()
///     .system_id(1)               // System `ID`
///     .component_id(1)            // Component `ID`
///     .connection(
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
pub type EdgeNode<V> = Node<Edge<V>, V, AsyncApi<V>>;

/// <sup>[`async`](crate::asnc)</sup>
/// Asynchronous node representing a MAVLink proxy.
pub type ProxyNode<V> = Node<Proxy, V, AsyncApi<V>>;
