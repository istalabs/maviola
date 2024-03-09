use std::marker::PhantomData;

use crate::asnc::io::ConnectionBuilder;
use crate::asnc::marker::AsyncConnConf;
use crate::core::io::ConnectionInfo;
use crate::core::marker::Unset;
use crate::core::utils::UniqueId;

use crate::prelude::*;

impl Network<Versionless, Unset> {
    /// <sup>[`async`](crate::asnc)</sup>
    /// Creates a network builder with empty configuration.
    pub fn asynchronous<V: MaybeVersioned>() -> Network<V, AsyncConnConf<V>> {
        Network {
            info: ConnectionInfo::Network,
            nodes: Default::default(),
            retry: Default::default(),
            _version: PhantomData,
        }
    }
}

impl<V: MaybeVersioned> Network<V, AsyncConnConf<V>> {
    /// <sup>[`async`](crate::asnc)</sup>
    /// Adds connection to a network.
    pub fn add_connection(
        mut self,
        conn_conf: impl ConnectionBuilder<V> + 'static,
    ) -> Network<V, AsyncConnConf<V>> {
        let node = Node::builder()
            .version::<V>()
            .async_connection(conn_conf)
            .conf();
        self.nodes.insert(UniqueId::new(), node);

        Network {
            nodes: self.nodes,
            ..self
        }
    }

    /// <sup>[`async`](crate::asnc)</sup>
    /// Adds a signed connection to a network.
    ///
    /// Signed connections will process incoming and outgoing frames according to corresponding
    /// signing strategies.
    pub fn add_signed_connection(
        mut self,
        conn_conf: impl ConnectionBuilder<V> + 'static,
        signer: FrameSigner,
    ) -> Network<V, AsyncConnConf<V>> {
        let node = Node::builder()
            .version::<V>()
            .async_connection(conn_conf)
            .signer(signer)
            .conf();
        self.nodes.insert(UniqueId::new(), node);

        Network {
            nodes: self.nodes,
            ..self
        }
    }
}

///////////////////////////////////////////////////////////////////////////////
//                                  Tests                                    //
///////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod async_network_tests {
    use super::*;

    use std::time::Duration;

    use crate::core::consts::SERVER_HANG_UP_TIMEOUT;
    use crate::core::io::Retry;
    use crate::core::utils::net::pick_unused_port;
    use crate::dialects::minimal::messages::Heartbeat;

    const RECONNECT_INTERVAL: Duration = SERVER_HANG_UP_TIMEOUT;
    // Should be at least 5-7 times bigger, than `RECONNECT_INTERVAL`, to make sure that tests will
    // run in parallel
    const WAIT_DURATION: Duration = Duration::from_millis(500);
    // Should be at least 10 times bigger, than `RECONNECT_INTERVAL`, to make sure that tests will
    // run in parallel
    const RECV_TIMEOUT: Duration = Duration::from_millis(750);

    async fn wait() {
        tokio::time::sleep(WAIT_DURATION).await;
    }

    #[tokio::test]
    async fn basic_network_workflow() {
        let addr_1 = format!("127.0.0.1:{}", pick_unused_port().unwrap());
        let addr_2 = format!("127.0.0.1:{}", pick_unused_port().unwrap());

        let network = Network::asynchronous()
            .add_connection(TcpServer::new(addr_1.as_str()).unwrap())
            .add_connection(TcpServer::new(addr_2.as_str()).unwrap());

        assert_eq!(network.nodes.len(), 2);

        let mut server = Node::builder()
            .version::<V2>()
            .id(MavLinkId::new(1, 0))
            .async_connection(network)
            .build()
            .await
            .unwrap();
        wait().await;

        let mut client_1 = Node::builder()
            .version::<V2>()
            .id(MavLinkId::new(1, 1))
            .async_connection(TcpClient::new(addr_1.as_str()).unwrap())
            .build()
            .await
            .unwrap();

        let mut client_2 = Node::builder()
            .version::<V2>()
            .id(MavLinkId::new(1, 2))
            .async_connection(TcpClient::new(addr_2.as_str()).unwrap())
            .build()
            .await
            .unwrap();

        server.send(&Heartbeat::default()).unwrap();

        let (frame, _) = client_1.recv_frame_timeout(RECV_TIMEOUT).await.unwrap();
        assert_eq!(frame.system_id(), 1);
        assert_eq!(frame.component_id(), 0);

        let (frame, _) = client_2.recv_frame_timeout(RECV_TIMEOUT).await.unwrap();
        assert_eq!(frame.system_id(), 1);
        assert_eq!(frame.component_id(), 0);

        client_1.send(&Heartbeat::default()).unwrap();
        wait().await;

        let (frame, _) = server.recv_frame_timeout(RECV_TIMEOUT).await.unwrap();
        assert_eq!(frame.system_id(), 1);
        assert_eq!(frame.component_id(), 1);
    }

    #[tokio::test]
    async fn network_reconnect() {
        let addr = format!("127.0.0.1:{}", pick_unused_port().unwrap());

        let server_conf = Node::builder()
            .version::<V2>()
            .id(MavLinkId::new(1, 0))
            .async_connection(TcpServer::new(addr.as_str()).unwrap())
            .conf();

        let server = Node::try_from_async_conf(server_conf.clone())
            .await
            .unwrap();
        wait().await;

        let network = Network::synchronous()
            .add_connection(TcpClient::new(addr.as_str()).unwrap())
            .retry(Retry::Always(RECONNECT_INTERVAL));
        let client = Node::builder()
            .version::<V2>()
            .id(MavLinkId::new(1, 1))
            .connection(network)
            .build()
            .unwrap();
        wait().await;

        drop(server);
        wait().await;
        let mut server = Node::try_from_async_conf(server_conf.clone())
            .await
            .unwrap();

        // This frame will be lost
        client.send(&Heartbeat::default()).unwrap();
        wait().await;

        client.send(&Heartbeat::default()).unwrap();
        server.recv_frame_timeout(RECV_TIMEOUT).await.unwrap();
    }
}
