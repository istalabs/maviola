//! # Maviola core errors
//!
//! These errors are returned by all `maviola` methods and functions.
//!
//! The top-level error is [`Error`]. Library API returns versions of this error possibly wrapping
//! other types of errors like [`FrameError`] or [`SpecError`].
//!
//! We also re-export errors from [`mavio::errors`](https://docs.rs/mavio/latest/mavio/errors/) and
//! wrap them as the corresponding variants of [`Error`]. All such low-level MAVLink abstractions
//! are available in [`crate::core`].

use std::fmt::{Debug, Formatter};
use std::sync::{mpsc, Arc, PoisonError};

use crate::protocol::MessageId;

/// <sup>[`mavio`](https://crates.io/crates/mavio)</sup>
#[doc(inline)]
pub use mavio::error::{
    ChecksumError, FrameError, IncompatFlagsError, SignatureError, SpecError, VersionError,
};

/// <sup>[`mavio`](https://crates.io/crates/mavio)</sup>
/// Low-level error re-exported from Mavio. Maviola wraps all variants of [`CoreError`] with its own
/// [`Error`] and provides proper conversions with [`From`] trait.
///
/// You may use mavio fallible functions such as [`Frame::add_signature`](mavio::Frame::add_signature)
/// with Maviola [`Result`] by calling [`Error::from`].
///
/// For example:
///
/// ```rust
/// use maviola::core::error::{CoreError, Error, Result};
///
/// fn core_fallible() -> core::result::Result<(), CoreError> {
///     Ok(())
/// }
///
/// fn fallible() -> Result<()> {
///     core_fallible().map_err(Error::from)
/// }
///
/// fallible().unwrap();
/// ```
/// ---
///
#[doc(inline)]
pub use crate::core::error::Error as CoreError;

/// Maviola result type.
pub type Result<T> = core::result::Result<T, Error>;

/// All errors generated by Maviola.
#[derive(Clone, Debug, thiserror::Error)]
pub enum Error {
    /// [`std::io::Error`] wrapper.
    #[error("I/O error: {0:?}")]
    Io(Arc<std::io::Error>),

    /// Frame encoding/decoding error.
    #[error("frame decoding/encoding error: {0:?}")]
    Frame(#[from] FrameError),

    /// Message encoding/decoding and specification discovery error.
    #[error("message decoding/encoding error: {0:?}")]
    Spec(SpecError),

    /// Node errors.
    #[error("node error: {0:?}")]
    Node(#[from] NodeError),

    /// Synchronisation errors.
    #[error("multi-threading error: {0:?}")]
    Sync(#[from] SyncError),

    /// Other errors.
    #[error("error: {0}")]
    Other(String),
}

/// Synchronisation errors.
#[derive(Clone, Debug, thiserror::Error)]
pub enum SyncError {
    /// Error while joining threads.
    #[error("error during thread join: {0:?}")]
    ThreadJoin(String),

    /// Failed due to a poisoned mutex.
    #[error("poisoned mutex: {0}")]
    PoisonedMutex(String),

    /// Attempt to read from an empty MPSC/MPMC channel.
    #[error("channel is empty")]
    Empty,

    /// Attempt to read or write into a closed MPSC/MPMC channel.
    #[error("channel is closed")]
    Disconnected,

    /// The receiver lagged too far behind. Attempting to receive again will
    /// return the oldest message still retained by the channel.
    ///
    /// Includes the number of skipped messages.
    #[error("receiver is too far behind: {0}")]
    Lagged(u64),

    /// This **channel** is currently empty, but the **Sender**(s) have not yet
    /// disconnected, so data may yet become available.
    #[error("timed out")]
    Timeout,
}

/// Node errors.
#[derive(Clone, Debug, thiserror::Error)]
pub enum NodeError {
    /// Transport no longer active error.
    #[error("transport is no longer active")]
    Inactive,

    /// Attempt to use a frame with message ID that can't be recognised by a dialect.
    #[error("provided frame with ID = {0} can't be decoded in current dialect {1}")]
    NotInDialect(MessageId, &'static str),
}

/// Error that happens, when caller attempts to send message to a closed channel.
///
/// The error wraps the value, that failed to be sent.
///
/// This error is returned by both synchronous and asynchronous channels.
pub struct SendError<T>(pub T);

/// Error that happens, when caller performs a blocking attempt to receive a message from a channel.
///
/// This error is returned by both synchronous and asynchronous channels.
#[derive(Clone, Copy, Debug, thiserror::Error)]
pub enum RecvError {
    /// Channel is disconnected, no messages will be received.
    #[error("channel is disconnected")]
    Disconnected,

    /// The receiver is far beyond the queue. Next request attempt will return the earliest possible
    /// message.
    #[error("lagged: {0}")]
    Lagged(u64),
}

/// Error that happens, when caller performs attempts to receive a message from a channel within a
/// timeout.
///
/// This error is returned by both synchronous and asynchronous channels.
#[derive(Clone, Copy, Debug, thiserror::Error)]
pub enum RecvTimeoutError {
    /// Channel is disconnected, no messages will be received.
    #[error("channel is disconnected")]
    Disconnected,

    /// This **channel** is currently empty, but the **Sender**(s) have not yet
    /// disconnected, so data may yet become available.
    #[error("timed out")]
    Timeout,

    /// The receiver is far beyond the queue. Next request attempt will return the earliest possible
    /// message.
    #[error("lagged: {0}")]
    Lagged(u64),
}

/// Error that happens, when caller tries to receive a message from a channel.
///
/// This error is returned by both synchronous and asynchronous channels.
#[derive(Clone, Copy, Debug, thiserror::Error)]
pub enum TryRecvError {
    /// Channel is empty.
    #[error("channel is empty")]
    Empty,

    /// Channel is disconnected, no messages will be received.
    #[error("channel is disconnected")]
    Disconnected,

    /// The receiver is far beyond the queue. Next request attempt will return the earliest possible
    /// message.
    #[error("lagged: {0}")]
    Lagged(u64),
}

impl From<mavio::error::Error> for Error {
    fn from(value: mavio::error::Error) -> Self {
        match value {
            mavio::error::Error::Io(err) => Self::Io(err),
            mavio::error::Error::Frame(err) => Self::Frame(err),
            mavio::error::Error::Spec(err) => Self::Spec(err),
            #[allow(deprecated)]
            mavio::error::Error::Buffer(err) => Self::Other(format!("{err:?}")),
        }
    }
}

impl From<SpecError> for Error {
    fn from(value: SpecError) -> Self {
        Error::Spec(value)
    }
}

impl<Guard> From<PoisonError<Guard>> for Error {
    fn from(value: PoisonError<Guard>) -> Self {
        Error::Sync(SyncError::PoisonedMutex(format!("{:?}", value)))
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::Io(Arc::new(value))
    }
}

impl From<VersionError> for Error {
    fn from(value: VersionError) -> Self {
        FrameError::from(value).into()
    }
}

impl From<ChecksumError> for Error {
    fn from(value: ChecksumError) -> Self {
        FrameError::from(value).into()
    }
}

impl From<SignatureError> for Error {
    fn from(value: SignatureError) -> Self {
        FrameError::from(value).into()
    }
}

impl From<IncompatFlagsError> for Error {
    fn from(value: IncompatFlagsError) -> Self {
        FrameError::from(value).into()
    }
}

///////////////////////////////////////////////////////////////////////////////
//                                Recv/Send                                  //
///////////////////////////////////////////////////////////////////////////////

impl<T> Debug for SendError<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SendError").finish_non_exhaustive()
    }
}

impl<T> From<SendError<T>> for Error {
    fn from(_: SendError<T>) -> Self {
        SyncError::Disconnected.into()
    }
}

impl From<RecvError> for Error {
    fn from(value: RecvError) -> Self {
        match value {
            RecvError::Disconnected => SyncError::Disconnected,
            RecvError::Lagged(n) => SyncError::Lagged(n),
        }
        .into()
    }
}

impl From<RecvTimeoutError> for Error {
    fn from(value: RecvTimeoutError) -> Self {
        match value {
            RecvTimeoutError::Disconnected => SyncError::Disconnected,
            RecvTimeoutError::Timeout => SyncError::Timeout,
            RecvTimeoutError::Lagged(n) => SyncError::Lagged(n),
        }
        .into()
    }
}

impl From<TryRecvError> for Error {
    fn from(value: TryRecvError) -> Self {
        match value {
            TryRecvError::Empty => SyncError::Empty,
            TryRecvError::Disconnected => SyncError::Disconnected,
            TryRecvError::Lagged(n) => SyncError::Lagged(n),
        }
        .into()
    }
}

///////////////////////////////////////////////////////////////////////////////
//                                   MPSC                                    //
///////////////////////////////////////////////////////////////////////////////

impl<T> From<mpsc::SendError<T>> for SendError<T> {
    fn from(value: mpsc::SendError<T>) -> Self {
        SendError(value.0)
    }
}

impl From<mpsc::RecvError> for RecvError {
    fn from(_: mpsc::RecvError) -> Self {
        RecvError::Disconnected
    }
}

impl From<mpsc::RecvTimeoutError> for RecvTimeoutError {
    fn from(value: mpsc::RecvTimeoutError) -> Self {
        match value {
            mpsc::RecvTimeoutError::Timeout => RecvTimeoutError::Timeout,
            mpsc::RecvTimeoutError::Disconnected => RecvTimeoutError::Disconnected,
        }
    }
}

impl From<mpsc::TryRecvError> for TryRecvError {
    fn from(value: mpsc::TryRecvError) -> Self {
        match value {
            mpsc::TryRecvError::Empty => TryRecvError::Empty,
            mpsc::TryRecvError::Disconnected => TryRecvError::Disconnected,
        }
    }
}

impl<T> From<mpsc::SendError<T>> for Error {
    fn from(_: mpsc::SendError<T>) -> Self {
        SyncError::Disconnected.into()
    }
}

impl From<mpsc::RecvError> for Error {
    fn from(_: mpsc::RecvError) -> Self {
        SyncError::Disconnected.into()
    }
}

impl From<mpsc::RecvTimeoutError> for Error {
    fn from(value: mpsc::RecvTimeoutError) -> Self {
        RecvTimeoutError::from(value).into()
    }
}

impl From<mpsc::TryRecvError> for Error {
    fn from(value: mpsc::TryRecvError) -> Self {
        TryRecvError::from(value).into()
    }
}

///////////////////////////////////////////////////////////////////////////////
//                              Tokio: Broadcast                             //
///////////////////////////////////////////////////////////////////////////////

impl<T> From<tokio::sync::broadcast::error::SendError<T>> for SendError<T> {
    fn from(value: tokio::sync::broadcast::error::SendError<T>) -> Self {
        Self(value.0)
    }
}

impl From<tokio::sync::broadcast::error::RecvError> for RecvError {
    fn from(value: tokio::sync::broadcast::error::RecvError) -> Self {
        match value {
            tokio::sync::broadcast::error::RecvError::Closed => RecvError::Disconnected,
            tokio::sync::broadcast::error::RecvError::Lagged(val) => RecvError::Lagged(val),
        }
    }
}

impl From<tokio::sync::broadcast::error::TryRecvError> for TryRecvError {
    fn from(value: tokio::sync::broadcast::error::TryRecvError) -> Self {
        match value {
            tokio::sync::broadcast::error::TryRecvError::Empty => TryRecvError::Empty,
            tokio::sync::broadcast::error::TryRecvError::Closed => TryRecvError::Disconnected,
            tokio::sync::broadcast::error::TryRecvError::Lagged(val) => TryRecvError::Lagged(val),
        }
    }
}

impl<T> From<tokio::sync::broadcast::error::SendError<T>> for Error {
    fn from(_: tokio::sync::broadcast::error::SendError<T>) -> Self {
        SyncError::Disconnected.into()
    }
}

impl From<tokio::sync::broadcast::error::RecvError> for Error {
    fn from(value: tokio::sync::broadcast::error::RecvError) -> Self {
        RecvError::from(value).into()
    }
}

impl From<tokio::sync::broadcast::error::TryRecvError> for Error {
    fn from(value: tokio::sync::broadcast::error::TryRecvError) -> Self {
        TryRecvError::from(value).into()
    }
}

///////////////////////////////////////////////////////////////////////////////
//                                Tokio: MPSC                                //
///////////////////////////////////////////////////////////////////////////////

impl<T> From<tokio::sync::mpsc::error::SendError<T>> for SendError<T> {
    fn from(value: tokio::sync::mpsc::error::SendError<T>) -> Self {
        SendError(value.0)
    }
}

impl<T> From<tokio::sync::mpsc::error::SendError<T>> for Error {
    fn from(value: tokio::sync::mpsc::error::SendError<T>) -> Self {
        SendError::from(value).into()
    }
}
