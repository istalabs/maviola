use std::cmp::min;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::sync::mpsc;

/// <sup>`⍚` | [`async`](crate::asnc)</sup>
/// Wrapper around [`mpsc::Receiver`] that implements [`AsyncRead`].
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

impl AsyncRead for MpscReader {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        if self.buf.len() >= buf.remaining() {
            let bytes_read = buf.remaining();
            buf.initialize_unfilled_to(bytes_read)[0..bytes_read]
                .copy_from_slice(&self.buf[0..bytes_read]);
            buf.advance(bytes_read);
            self.get_mut().buf.drain(0..bytes_read);
            return Poll::Ready(Ok(()));
        }

        let mut recv_buf = {
            let mut pinned = std::pin::pin!(self.as_mut().get_mut().receiver.recv());
            let poll_result = pinned.as_mut().poll(cx);

            let recv_buf = match poll_result {
                Poll::Ready(recv_buf) => match recv_buf {
                    None => {
                        return Poll::Ready(Err(std::io::Error::new(
                            std::io::ErrorKind::ConnectionAborted,
                            "MpscReader: channel closed",
                        )))
                    }
                    Some(recv_buf) => recv_buf,
                },
                Poll::Pending => return Poll::Pending,
            };

            recv_buf
        };
        self.as_mut().buf.append(&mut recv_buf);

        let bytes_read = min(self.buf.len(), buf.remaining());
        buf.initialize_unfilled_to(bytes_read)[0..bytes_read]
            .copy_from_slice(&self.buf[0..bytes_read]);
        buf.advance(bytes_read);
        self.buf.drain(0..bytes_read);

        Poll::Ready(Ok(()))
    }
}

/// <sup>`⍚` | [`async`](crate::asnc)</sup>
/// Wrapper around [`mpsc::Sender`] that implements [`AsyncWrite`].
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

impl AsyncWrite for MpscWriter {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        let mut pinned = std::pin::pin!(self.sender.send(buf.to_vec()));
        let poll_result = pinned.as_mut().poll(cx);

        match poll_result {
            Poll::Ready(send_result) => match send_result {
                Ok(_) => Poll::Ready(Ok(buf.len())),
                Err(_) => Poll::Ready(Err(std::io::Error::new(
                    std::io::ErrorKind::ConnectionAborted,
                    "MpscWriter: channel closed",
                ))),
            },
            Poll::Pending => Poll::Pending,
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        Poll::Ready(Ok(()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn async_mpsc_writer_basic_workflow() {
        let (write_tx, mut write_rx) = mpsc::channel(1024);
        let mut writer = MpscWriter::new(write_tx);

        writer.write_all(&[1u8; 10]).await.unwrap();
        writer.write_all(&[2u8; 10]).await.unwrap();
        writer.write_all(&[3u8; 10]).await.unwrap();

        assert_eq!(write_rx.recv().await.unwrap(), vec![1u8; 10]);
        assert_eq!(write_rx.recv().await.unwrap(), vec![2u8; 10]);
        assert_eq!(write_rx.recv().await.unwrap(), vec![3u8; 10]);
    }

    #[tokio::test]
    async fn async_mpsc_reader_basic_workflow() {
        let (read_tx, read_rx) = mpsc::channel(1024);
        let mut reader = MpscReader::new(read_rx);

        read_tx.send(vec![1u8; 10]).await.unwrap();
        read_tx.send(vec![2u8; 10]).await.unwrap();
        read_tx.send(vec![3u8; 10]).await.unwrap();

        let mut buf = [0u8; 10];

        reader.read_exact(&mut buf).await.unwrap();
        assert_eq!(buf, [1u8; 10]);
        reader.read_exact(&mut buf).await.unwrap();
        assert_eq!(buf, [2u8; 10]);
        reader.read_exact(&mut buf).await.unwrap();
        assert_eq!(buf, [3u8; 10]);
    }
}
