use std::net::TcpListener;

use crate::core::io::ChannelInfo;
use crate::core::utils::Closer;
use crate::sync::consts::{TCP_READ_TIMEOUT, TCP_WRITE_TIMEOUT};
use crate::sync::io::{Connection, ConnectionBuilder, ConnectionHandler};

use crate::prelude::*;

impl<V: MaybeVersioned + 'static> ConnectionBuilder<V> for TcpServer {
    fn build(&self) -> Result<(Connection<V>, ConnectionHandler)> {
        let server_addr = self.addr;
        let listener = TcpListener::bind(self.addr)?;

        let conn_state = Closer::new();
        let (connection, chan_factory) = Connection::new(self.info.clone(), conn_state.to_shared());

        let handler = ConnectionHandler::spawn(move || -> Result<()> {
            for stream in listener.incoming() {
                if conn_state.is_closed() {
                    break;
                }

                let stream = stream?;
                let peer_addr = stream.peer_addr()?;
                let writer = stream;
                let reader = writer.try_clone()?;

                writer.set_write_timeout(TCP_WRITE_TIMEOUT)?;
                writer.set_read_timeout(TCP_READ_TIMEOUT)?;

                let channel = chan_factory.build(
                    ChannelInfo::TcpServer {
                        server_addr,
                        peer_addr,
                    },
                    reader,
                    writer,
                );
                channel.spawn().discard();
            }

            Ok(())
        });

        Ok((connection, handler))
    }
}
