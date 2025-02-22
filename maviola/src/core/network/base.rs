use std::collections::HashMap;
use std::fmt::Debug;
use std::marker::PhantomData;

use crate::core::io::{ConnectionConf, ConnectionInfo, RetryStrategy};
use crate::core::marker::{MaybeConnConf, NodeKind, Proxy};
use crate::core::node::{IntoNodeConf, NodeConf};
use crate::core::utils::UniqueId;

use crate::prelude::*;

/// MAVLink network, a collection of nodes with different underlying transports.
///
/// Each message received by one node will be broadcast to other nodes. More specifically, the
/// broadcast operates on the level of channels. That means, that if, for example, a server node
/// receives a message from one of its clients, then this message will be forwarded to all other
/// clients of this server and all other nodes.
///
/// # Examples
///
/// Create a synchronous node with a network containing two TCP servers:
///
/// ```rust,no_run
/// # #[cfg(feature = "sync")] {
/// use std::time::Duration;
/// use maviola::core::io::RetryStrategy;
///
/// use maviola::prelude::*;
/// use maviola::sync::prelude::*;
///
/// let node = Node::sync::<V2>()
///     .id(MavLinkId::new(1, 17))
///     .connection(
///         Network::sync()
///             // We can either specify a connection
///             .add_connection(TcpServer::new("127.0.0.1:5600").unwrap())
///             // Or the entire proxy node configuration
///             .add_node(
///                 Node::sync()
///                     .connection(TcpServer::new("127.0.0.1:5601").unwrap())
///                     /* other node configuration */
///             )
///             // Attempt to repair disconnected nodes
///             .retry(RetryStrategy::Attempts(10, Duration::from_secs(2)))
///             // Stop if at least one node is down and all retry attempts have failed
///             .stop_on_node_down(true)
///     )
///     .build().unwrap();
/// # }
/// ```
///
/// Create an asynchronous node with a network containing a TCP server and a TCP client:
///
/// ```rust,no_run
/// # #[cfg(not(feature = "async"))] fn main() {}
/// # #[cfg(feature = "async")]
/// # #[tokio::main] async fn main() {
/// use std::time::Duration;
/// use maviola::core::io::RetryStrategy;
///
/// use maviola::prelude::*;
/// use maviola::asnc::prelude::*;
///
/// let node = Node::asnc::<V2>()
///     .id(MavLinkId::new(1, 17))
///     .connection(
///         Network::asnc()
///             // We can either specify a connection
///             .add_connection(TcpServer::new("127.0.0.1:5600").unwrap())
///             // Or the entire proxy node configuration
///             .add_node(
///                 Node::asnc()
///                     .connection(TcpClient::new("127.0.0.1:5601").unwrap())
///                     /* other node configuration */
///             )
///             // Attempt to repair disconnected nodes
///             .retry(RetryStrategy::Attempts(10, Duration::from_secs(2)))
///             // Stop if at least one node is down and all retry attempts have failed
///             .stop_on_node_down(true)
///     )
///     .build().await.unwrap();
/// # }
/// ```
#[derive(Debug)]
pub struct Network<V: MaybeVersioned, C: MaybeConnConf> {
    pub(crate) info: ConnectionInfo,
    pub(crate) nodes: HashMap<UniqueId, NodeConf<Proxy, V, C>>,
    pub(crate) retry: RetryStrategy,
    pub(crate) stop_on_node_down: bool,
    pub(crate) _version: PhantomData<V>,
}

impl<V: MaybeVersioned, C: MaybeConnConf> Network<V, C> {
    /// Adds node configuration to the network.
    ///
    /// Accepts anything that can be converted into a [`NodeConf`].
    ///
    /// Make sure, that node configuration was built with the protocol version, that matches target
    /// node. In particular, do not forget to set [`NodeBuilder::version`], if you are using a
    /// versioned node.
    ///
    /// [`NodeBuilder::version`]: crate::core::node::NodeBuilder::version
    pub fn add_node<K: NodeKind>(mut self, node: impl IntoNodeConf<K, V, C>) -> Self {
        let node = node.into_node_conf().into_proxy();
        self.nodes.insert(UniqueId::new(), node);
        self
    }

    /// Defines retry strategy for a network.
    ///
    /// When node goes down and it [`NodeConf::is_repairable`], then network will attempt to restore
    /// a node according to a specified strategy.
    ///
    /// When [`Self::retry`] is set to `true`, then the entire network will be disconnected, when
    /// at least one node is stopped and can't be repaired.
    pub fn retry(mut self, retry: RetryStrategy) -> Self {
        self.retry = retry;
        self
    }

    /// Defines, whether entire network should go down, when one of the nodes is disconnected.
    ///
    /// This option works in conjunction with [`Self::retry`]. The node will be considered down,
    /// if all retry attempts has failed.
    pub fn stop_on_node_down(mut self, value: bool) -> Self {
        self.stop_on_node_down = value;
        self
    }
}

impl<V: MaybeVersioned, C: MaybeConnConf> ConnectionConf for Network<V, C> {
    fn info(&self) -> &ConnectionInfo {
        &self.info
    }
}
