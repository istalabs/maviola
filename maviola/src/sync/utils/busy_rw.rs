//! # Busy reader and writer
//!
//! Implementation for [`Read`] and [`Write`] that always returns [`ErrorKind::TimedOut`].

use std::io::{Error, ErrorKind, Read, Write};

/// Reader that always returns a time-out error.
///
/// This reader always returns time-out I/O error for any read attempt. It is useful for creating
/// uni-directional reader / writer pairs. For example, a file writer should newer read and vice
/// versa.
///
/// The writer counterpart is [`BusyWriter`].
pub struct BusyReader;

impl Read for BusyReader {
    /// Always returns an error.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::TimedOut`] for all reading attempts.
    fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
        Err(Error::new(
            ErrorKind::TimedOut,
            "attempt to read from NoReader",
        ))
    }

    /// Always returns an error.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::TimedOut`] for all reading attempts.
    fn read_exact(&mut self, _: &mut [u8]) -> std::io::Result<()> {
        Err(Error::new(
            ErrorKind::TimedOut,
            "attempt to read from NoReader",
        ))
    }
}

/// Writer that always returns a time-out error.
///
/// This writer always returns time-out I/O error for any write attempt. It is useful for creating
/// uni-directional reader / writer pairs. For example, a file reader should newer read and vice
/// versa.
///
/// The reader counterpart is [`BusyReader`].
pub struct BusyWriter;

impl Write for BusyWriter {
    /// Always returns a timed-out error.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::TimedOut`] for all writing attempts.
    fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
        Err(Error::new(
            ErrorKind::TimedOut,
            "attempt to write to NoWriter",
        ))
    }

    /// Flush this output stream, ensuring that all intermediately buffered contents reach their destination.
    ///
    /// Always returns [`Ok`] since [`BusyWriter`] never writes.
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }

    fn by_ref(&mut self) -> &mut Self
    where
        Self: Sized,
    {
        self
    }
}
