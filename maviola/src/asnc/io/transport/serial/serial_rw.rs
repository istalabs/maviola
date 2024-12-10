use std::io::{Error, ErrorKind};
use std::pin::Pin;
use std::sync::Mutex;
use std::sync::{Arc, TryLockError};
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio_serial::SerialStream;

/// A wrapper around [`SerialStream`] that implements [`AsyncRead`] and [`AsyncWrite`] and can be
/// cloned.
#[derive(Clone)]
pub struct SerialRW {
    port: Arc<Mutex<SerialStream>>,
}

impl SerialRW {
    /// Creates a new serial port reader/writer.
    pub fn new(port: SerialStream) -> Self {
        Self {
            port: Arc::new(Mutex::new(port)),
        }
    }
}

impl AsyncRead for SerialRW {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        match self.port.try_lock() {
            Ok(mut port) => {
                let mut inner_buf = vec![0; buf.remaining()];
                match port.try_read(&mut inner_buf.as_mut_slice()) {
                    Ok(bytes_read) => {
                        buf.put_slice(&inner_buf.as_slice()[0..bytes_read]);
                        Poll::Ready(Ok(()))
                    }
                    Err(err) => match err.kind() {
                        ErrorKind::WouldBlock => {
                            cx.waker().wake_by_ref();
                            Poll::Pending
                        }
                        _ => Poll::Ready(Err(err)),
                    },
                }
            }
            Err(_) => Poll::Pending,
        }
    }
}

impl AsyncWrite for SerialRW {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        match self.port.try_lock() {
            Ok(mut port) => match port.try_write(buf) {
                Ok(bytes_sent) => Poll::Ready(Ok(bytes_sent)),
                Err(err) => match err.kind() {
                    ErrorKind::WouldBlock => {
                        cx.waker().wake_by_ref();
                        Poll::Pending
                    }
                    _ => Poll::Ready(Err(err)),
                },
            },
            Err(_) => Poll::Pending,
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        match self.port.try_lock() {
            Ok(mut port) => {
                let pinned = Pin::new(&mut *port);
                pinned.poll_flush(cx)
            }
            Err(err) => match err {
                TryLockError::Poisoned(err) => {
                    Poll::Ready(Err(Error::new(ErrorKind::Other, format!("{:?}", err))))
                }
                TryLockError::WouldBlock => {
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
            },
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        match self.port.try_lock() {
            Ok(mut port) => {
                let pinned = Pin::new(&mut *port);
                pinned.poll_shutdown(cx)
            }
            Err(err) => match err {
                TryLockError::Poisoned(err) => {
                    Poll::Ready(Err(Error::new(ErrorKind::Other, format!("{:?}", err))))
                }
                TryLockError::WouldBlock => {
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
            },
        }
    }
}
