use async_trait::async_trait;
use tokio::net::UdpSocket;

use crate::asnc::io::transport::udp::udp_rw::UdpRW;
use crate::asnc::io::{Connection, ConnectionBuilder, ConnectionHandler};
use crate::asnc::marker::AsyncConnConf;
use crate::core::io::ChannelInfo;
use crate::core::utils::net::{pick_unused_port, resolve_socket_addr};
use crate::core::utils::SharedCloser;

use crate::prelude::*;

#[async_trait]
impl<V: MaybeVersioned + 'static> ConnectionBuilder<V> for UdpClient {
    async fn build(&self) -> Result<(Connection<V>, ConnectionHandler)> {
        let bind_addr = match self.bind_addr {
            None => resolve_socket_addr(format!("{}:{}", self.host, pick_unused_port()?))?,
            Some(bind_addr) => bind_addr,
        };
        let server_addr = self.addr;

        let udp_socket = UdpSocket::bind(bind_addr).await?;
        udp_socket.connect(server_addr).await?;

        let writer = UdpRW::new(udp_socket);
        let reader = writer.clone();

        let (connection, chan_factory) = Connection::new(self.info.clone(), SharedCloser::new());

        let channel = chan_factory.build(
            ChannelInfo::UdpClient {
                server_addr,
                bind_addr,
            },
            reader,
            writer,
        );
        let channel_state = channel.spawn().await;

        let handler = ConnectionHandler::spawn_from_state(channel_state);

        Ok((connection, handler))
    }

    fn to_conf(&self) -> AsyncConnConf<V> {
        AsyncConnConf::new(self.clone())
    }

    fn is_repairable(&self) -> bool {
        true
    }
}
