use std::net::{SocketAddr, TcpListener, TcpStream};
use std::thread;

use crate::core::consts::SERVER_HANG_UP_TIMEOUT;
use crate::core::io::ChannelInfo;
use crate::core::utils::{Closable, Closer};
use crate::sync::consts::{TCP_READ_TIMEOUT, TCP_WRITE_TIMEOUT};
use crate::sync::io::{Connection, ConnectionBuilder, ConnectionHandler};
use crate::sync::marker::ConnConf;

use crate::prelude::*;

impl<V: MaybeVersioned + 'static> ConnectionBuilder<V> for TcpServer {
    fn build(&self) -> Result<(Connection<V>, ConnectionHandler)> {
        let server_addr = self.addr;
        let listener = TcpListener::bind(self.addr)?;

        let conn_state = Closer::new();
        let (connection, chan_factory) = Connection::new(self.info.clone(), conn_state.to_shared());

        let handler = ConnectionHandler::spawn(move || -> Result<()> {
            on_close_handler(conn_state.to_closable(), server_addr);

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

    fn to_conf(&self) -> ConnConf<V> {
        ConnConf::new(self.clone())
    }
}

fn on_close_handler(state: Closable, addr: SocketAddr) {
    thread::spawn(move || {
        while !state.is_closed() {
            thread::sleep(SERVER_HANG_UP_TIMEOUT);
        }
        _ = TcpStream::connect(addr);
    });
}
