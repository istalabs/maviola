use std::future::Future;
use std::io::{Error, ErrorKind};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::UdpSocket;

/// A wrapper around [`UdpSocket`] that implements [`AsyncRead`] and [`AsyncWrite`].
#[derive(Clone)]
pub struct UdpRW {
    socket: Arc<UdpSocket>,
}

impl UdpRW {
    /// Creates a new UDP reader/writer.
    pub fn new(socket: UdpSocket) -> Self {
        Self {
            socket: Arc::new(socket),
        }
    }
}

impl AsyncRead for UdpRW {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        match self.socket.try_recv_buf(buf) {
            Ok(_) => Poll::Ready(Ok(())),
            Err(err) => match err.kind() {
                ErrorKind::WouldBlock => {
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
                _ => Poll::Ready(Err(err)),
            },
        }
    }
}

impl AsyncWrite for UdpRW {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        match self.socket.try_send(buf) {
            Ok(bytes_sent) => Poll::Ready(Ok(bytes_sent)),
            Err(err) => match err.kind() {
                ErrorKind::WouldBlock => {
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
                _ => Poll::Ready(Err(err)),
            },
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::utils::net::pick_unused_port;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn async_udp_write_workflow() {
        let bind_port = pick_unused_port().unwrap();
        let bind_addr = format!("127.0.0.1:{bind_port}");
        let server_socket = UdpSocket::bind(bind_addr.as_str()).await.unwrap();

        let client_bind_port = pick_unused_port().unwrap();
        let client_bind_addr = format!("127.0.0.1:{client_bind_port}");
        let client_socket = UdpSocket::bind(client_bind_addr).await.unwrap();
        client_socket.connect(bind_addr.as_str()).await.unwrap();

        let mut udp_rw = UdpRW::new(client_socket);
        udp_rw.write_all(&[1u8; 10]).await.unwrap();

        let mut buf = [0u8; 10];
        server_socket.recv_from(&mut buf).await.unwrap();
        assert_eq!(buf, [1u8; 10]);
    }

    #[tokio::test]
    async fn async_udp_read_workflow() {
        let bind_port = pick_unused_port().unwrap();
        let bind_addr = format!("127.0.0.1:{bind_port}");
        let server_socket = UdpSocket::bind(bind_addr.as_str()).await.unwrap();

        let client_bind_port = pick_unused_port().unwrap();
        let client_bind_addr = format!("127.0.0.1:{client_bind_port}");
        let client_socket = UdpSocket::bind(client_bind_addr.as_str()).await.unwrap();
        client_socket.connect(bind_addr.as_str()).await.unwrap();
        let mut udp_rw = UdpRW::new(client_socket);

        server_socket
            .send_to(&[1u8; 10], client_bind_addr.as_str())
            .await
            .unwrap();

        let mut buf = [0u8; 10];
        udp_rw.read_exact(&mut buf).await.unwrap();
        assert_eq!(buf, [1u8; 10]);
    }
}
