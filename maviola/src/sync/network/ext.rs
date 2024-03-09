use std::marker::PhantomData;

use crate::core::io::ConnectionInfo;
use crate::core::marker::Unset;
use crate::core::utils::UniqueId;
use crate::sync::io::ConnectionBuilder;
use crate::sync::marker::ConnConf;

use crate::prelude::*;

impl Network<Versionless, Unset> {
    /// <sup>[`sync`](crate::sync)</sup>
    /// Creates a network builder with empty configuration.
    pub fn synchronous<V: MaybeVersioned>() -> Network<V, ConnConf<V>> {
        Network {
            info: ConnectionInfo::Network,
            nodes: Default::default(),
            retry: Default::default(),
            _version: PhantomData,
        }
    }
}

impl<V: MaybeVersioned> Network<V, ConnConf<V>> {
    /// <sup>[`sync`](crate::sync)</sup>
    /// Adds connection to a network.
    pub fn add_connection(
        mut self,
        conn_conf: impl ConnectionBuilder<V> + 'static,
    ) -> Network<V, ConnConf<V>> {
        let node = Node::builder().version::<V>().connection(conn_conf).conf();
        self.nodes.insert(UniqueId::new(), node);

        Network {
            nodes: self.nodes,
            ..self
        }
    }

    /// <sup>[`sync`](crate::sync)</sup>
    /// Adds a signed connection to a network.
    ///
    /// Signed connections will process incoming and outgoing frames according to corresponding
    /// signing strategies.
    pub fn add_signed_connection(
        mut self,
        conn_conf: impl ConnectionBuilder<V> + 'static,
        signer: FrameSigner,
    ) -> Network<V, ConnConf<V>> {
        let node = Node::builder()
            .version::<V>()
            .connection(conn_conf)
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
mod sync_network_tests {
    use super::*;

    use std::thread;
    use std::time::Duration;

    use crate::core::consts::SERVER_HANG_UP_TIMEOUT;
    use crate::core::io::Retry;
    use crate::core::utils::net::pick_unused_port;
    use crate::dialects::minimal::messages::Heartbeat;

    const RECONNECT_INTERVAL: Duration = SERVER_HANG_UP_TIMEOUT;
    // Should be at least twice as big, as `RECONNECT_INTERVAL`
    const WAIT_DURATION: Duration = Duration::from_millis(100);

    fn wait() {
        thread::sleep(WAIT_DURATION);
    }

    #[test]
    fn basic_network_workflow() {
        let addr_1 = format!("127.0.0.1:{}", pick_unused_port().unwrap());
        let addr_2 = format!("127.0.0.1:{}", pick_unused_port().unwrap());

        let network = Network::synchronous()
            .add_node(
                Node::builder()
                    .version::<V2>()
                    .connection(TcpServer::new(addr_1.as_str()).unwrap()),
            )
            .add_connection(TcpServer::new(addr_2.as_str()).unwrap());

        assert_eq!(network.nodes.len(), 2);

        let server = Node::builder()
            .version::<V2>()
            .id(MavLinkId::new(1, 0))
            .connection(network)
            .build()
            .unwrap();
        wait();

        let client_1 = Node::builder()
            .version::<V2>()
            .id(MavLinkId::new(1, 1))
            .connection(TcpClient::new(addr_1.as_str()).unwrap())
            .build()
            .unwrap();

        let client_2 = Node::builder()
            .version::<V2>()
            .id(MavLinkId::new(1, 2))
            .connection(TcpClient::new(addr_2.as_str()).unwrap())
            .build()
            .unwrap();

        server.send(&Heartbeat::default()).unwrap();

        let (frame, _) = client_1.recv_frame_timeout(WAIT_DURATION).unwrap();
        assert_eq!(frame.system_id(), 1);
        assert_eq!(frame.component_id(), 0);

        let (frame, _) = client_2.recv_frame_timeout(WAIT_DURATION).unwrap();
        assert_eq!(frame.system_id(), 1);
        assert_eq!(frame.component_id(), 0);

        client_1.send(&Heartbeat::default()).unwrap();

        let (frame, _) = server.recv_frame_timeout(WAIT_DURATION).unwrap();
        assert_eq!(frame.system_id(), 1);
        assert_eq!(frame.component_id(), 1);
    }

    #[test]
    fn network_reconnect() {
        let addr = format!("127.0.0.1:{}", pick_unused_port().unwrap());

        let server_conf = Node::builder()
            .version::<V2>()
            .id(MavLinkId::new(1, 0))
            .connection(TcpServer::new(addr.as_str()).unwrap())
            .conf();

        let server = Node::try_from_conf(server_conf.clone()).unwrap();
        wait();

        let network = Network::synchronous()
            .add_connection(TcpClient::new(addr.as_str()).unwrap())
            .retry(Retry::Always(RECONNECT_INTERVAL));
        let client = Node::builder()
            .version::<V2>()
            .id(MavLinkId::new(1, 1))
            .connection(network)
            .build()
            .unwrap();
        wait();

        drop(server);
        wait();
        let server = Node::try_from_conf(server_conf.clone()).unwrap();

        // This frame will be lost
        client.send(&Heartbeat::default()).unwrap();
        wait();

        client.send(&Heartbeat::default()).unwrap();
        server.recv_frame_timeout(WAIT_DURATION).unwrap();
    }
}