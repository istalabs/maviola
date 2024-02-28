//! # Asynchronous busy reader and writer
//!
//! Implementation for [`AsyncRead`] and [`AsyncWrite`] that always returns [`ErrorKind::TimedOut`].

use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, Error, ErrorKind, ReadBuf};

/// <sup>`⍚` | [`sync`](crate::sync)</sup>
/// Reader that always returns a time-out error.
///
/// This reader always returns time-out I/O error for any read attempt. It is useful for creating
/// uni-directional reader / writer pairs. For example, a file writer should newer read and vice
/// versa.
///
/// The writer counterpart is [`BusyWriter`].
pub struct BusyReader;

impl AsyncRead for BusyReader {
    /// Always returns an error.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::TimedOut`] for all reading attempts.
    fn poll_read(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        _: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Poll::Ready(Err(Error::new(
            ErrorKind::TimedOut,
            "attempt to read from BusyReader",
        )))
    }
}

/// <sup>`⍚` | [`sync`](crate::sync)</sup>
/// Writer that always returns a time-out error.
///
/// This writer always returns time-out I/O error for any write attempt. It is useful for creating
/// uni-directional reader / writer pairs. For example, a file reader should newer read and vice
/// versa.
///
/// The reader counterpart is [`BusyReader`].
pub struct BusyWriter;

impl AsyncWrite for BusyWriter {
    /// Always returns an error.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::TimedOut`] for all writing attempts.
    fn poll_write(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        _: &[u8],
    ) -> Poll<Result<usize, Error>> {
        Poll::Ready(Err(Error::new(
            ErrorKind::TimedOut,
            "attempt to write to BusyWriter",
        )))
    }

    /// Flush this output stream, ensuring that all intermediately buffered contents reach their destination.
    ///
    /// Always returns [`Ok`] since [`BusyWriter`] never actually writes.
    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }

    /// Initiates or attempts to shut down this writer, returning success when the I/O connection
    /// has completely shut down.
    ///
    /// Always returns [`Ok`] since [`BusyWriter`] is never connected.
    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }
}
