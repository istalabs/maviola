use std::cmp::min;
use std::io::{Read, Write};
use std::sync::mpsc;

/// <sup>`⍚` | [`sync`](crate::sync)</sup>
/// Wrapper around [`mpsc::Receiver`] that implements [`Read`].
///
/// When channel is closed, [`MpscReader`] returns
/// [`ErrorKind::ConnectionAborted`](std::io::ErrorKind::ConnectionAborted).
#[derive(Debug)]
pub struct MpscReader {
    receiver: mpsc::Receiver<Vec<u8>>,
    buf: Vec<u8>,
}

impl MpscReader {
    /// Creates a new [`MpscReader`] from [`mpsc::Receiver`].
    pub fn new(receiver: mpsc::Receiver<Vec<u8>>) -> Self {
        Self {
            receiver,
            buf: Vec::new(),
        }
    }
}

impl Read for MpscReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.buf.len() >= buf.len() {
            let bytes_read = buf.len();
            buf[0..bytes_read].copy_from_slice(&self.buf[0..bytes_read]);
            self.buf.drain(0..bytes_read);
            return Ok(bytes_read);
        }

        {
            let mut recv_buf = self
                .receiver
                .recv()
                .map_err(|err| std::io::Error::new(std::io::ErrorKind::ConnectionAborted, err))?;
            self.buf.append(&mut recv_buf);
        }

        let bytes_read = min(self.buf.len(), buf.len());
        buf[0..bytes_read].copy_from_slice(&self.buf[0..bytes_read]);
        self.buf.drain(0..bytes_read);

        Ok(bytes_read)
    }
}

/// <sup>`⍚` | [`sync`](crate::sync)</sup>
/// Wrapper around [`mpsc::Sender`] that implements [`Write`].
///
/// When channel is closed, [`MpscWriter`] returns
/// [`ErrorKind::ConnectionAborted`](std::io::ErrorKind::ConnectionAborted).
#[derive(Clone, Debug)]
pub struct MpscWriter {
    sender: mpsc::Sender<Vec<u8>>,
}

impl MpscWriter {
    /// Creates a new [`MpscWriter`] from [`mpsc::Sender`].
    pub fn new(sender: mpsc::Sender<Vec<u8>>) -> Self {
        Self { sender }
    }
}

impl Write for MpscWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.sender.send(buf.to_vec()).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::ConnectionAborted,
                "MpscWriter: channel closed",
            )
        })?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
