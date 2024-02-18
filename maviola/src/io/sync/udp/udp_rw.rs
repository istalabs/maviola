use std::io::{Read, Write};
use std::net::UdpSocket;
use std::thread;

use crate::consts::{UDP_RETRIES, UDP_RETRY_INTERVAL};

/// A wrapper around [`UdpSocket`] that implements [`Read`] and [`Write`].
pub struct UdpRW {
    socket: UdpSocket,
}

impl UdpRW {
    /// Creates a new UDP reader/writer.
    pub fn new(socket: UdpSocket) -> Self {
        Self { socket }
    }

    /// Creates a new independently owned handle to the underlying socket.
    ///
    /// This is a thin wrapper around [`UdpSocket::try_clone`].
    pub fn try_clone(&self) -> std::io::Result<Self> {
        Ok(Self {
            socket: self.socket.try_clone()?,
        })
    }
}

impl Read for UdpRW {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.socket.recv(buf)
    }
}

impl Write for UdpRW {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut res = Ok(0);
        for i in 0..UDP_RETRIES {
            res = self.socket.send(buf);
            if res.is_ok() || i == UDP_RETRIES - 1 {
                break;
            }
            thread::sleep(UDP_RETRY_INTERVAL);
        }
        res
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
