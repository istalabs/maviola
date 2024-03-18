use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::thread;

use crate::core::consts::SERVER_HANG_UP_TIMEOUT;
use crate::core::io::{ChannelInfo, ConnectionConf, ConnectionInfo};
use crate::core::utils::{Closable, Closer};
use crate::sync::consts::{SOCK_ACCEPT_INTERVAL, SOCK_READ_TIMEOUT, SOCK_WRITE_TIMEOUT};
use crate::sync::io::{Connection, ConnectionBuilder, ConnectionHandler};

use crate::prelude::*;
use crate::sync::marker::ConnConf;

impl<V: MaybeVersioned + 'static> ConnectionBuilder<V> for SockServer {
    fn build(&self) -> Result<(Connection<V>, ConnectionHandler)> {
        let path = self.path.clone();
        let listener = UnixListener::bind(self.path.as_path())?;
        listener.set_nonblocking(true)?;

        let conn_state = Closer::new();
        let (connection, chan_factory) = Connection::new(self.info.clone(), conn_state.to_shared());

        let info = self.info().clone();

        let handler = ConnectionHandler::spawn(move || -> Result<()> {
            on_close_handler(conn_state.to_closable(), path.clone(), info);

            while !conn_state.is_closed() {
                let writer = match listener.accept() {
                    Ok((stream, _)) => stream,
                    Err(err) => match err.kind() {
                        std::io::ErrorKind::WouldBlock => {
                            thread::sleep(SOCK_ACCEPT_INTERVAL);
                            continue;
                        }
                        _ => return Err(err.into()),
                    },
                };
                let reader = writer.try_clone()?;

                writer.set_nonblocking(false)?;
                writer.set_write_timeout(SOCK_WRITE_TIMEOUT)?;
                reader.set_read_timeout(SOCK_READ_TIMEOUT)?;

                let channel = chan_factory.build(
                    ChannelInfo::SockServer { path: path.clone() },
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

fn on_close_handler(state: Closable, path: PathBuf, info: ConnectionInfo) {
    thread::spawn(move || {
        while !state.is_closed() {
            thread::sleep(SERVER_HANG_UP_TIMEOUT);
        }

        log::debug!("[{info:?}] spawn wake-up connection to close server listening loop");
        _ = UnixStream::connect(path.as_path());
    });
}
