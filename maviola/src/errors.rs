//! # Maviola errors
//!
//! These errors are returned by all `maviola` methods and functions.
//!
//! The top-level error is [`Error`]. Library API returns versions of this error possibly wrapping
//! other types of errors like [`FrameError`] or [`SpecError`].
//!
//! We also re-export errors from [`mavio::errors`](https://docs.rs/mavio/latest/mavio/errors/) and
//! wrap them as the corresponding variants of [`Error`].

use std::sync::{mpsc, Arc, PoisonError};

use mavio::protocol::MessageId;

/// <sup>From [`mavio`](https://docs.rs/mavio/0.2.0-rc2/mavio/errors/)</sup>
#[doc(inline)]
pub use mavio::errors::{FrameError, SpecError};

/// <sup>From [`mavio`](https://docs.rs/mavio/latest/mavio/errors/)</sup>
/// Re-exported from [`mavio::errors::Error`](https://docs.rs/mavio/0.2.0-rc2/mavio/errors/enum.Error.html).
/// Maviola wraps all variants of [`CoreError`] with its own [`Error`] and provides a proper
/// conversion.
///
/// You may use mavio fallible functions such as [`Frame`](mavio::Frame::add_signature)
/// with Maviola [`Result`] by calling [`Error::from`].
///
/// For example:
///
/// ```rust
/// use maviola::errors::{CoreError, Error, Result};
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
pub use mavio::errors::Error as CoreError;

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

    /// Multi-threading errors.
    #[error("multi-threading error: {0:?}")]
    Sync(#[from] SyncError),

    /// Other errors.
    #[error("error: {0}")]
    Other(String),
}

/// Multi-threading errors.
#[derive(Clone, Debug, thiserror::Error)]
pub enum SyncError {
    /// Error while joining threads.
    #[error("error during thread join: {0:?}")]
    ThreadJoin(String),

    /// Failed due to poisoned mutex.
    #[error("poisoned mutex: {0}")]
    PoisonedMutex(String),

    /// Attempt to read or write into a closed MPSC/MPMC channel.
    #[error("channel error: {0}")]
    ChannelClosed(String),

    /// Error during non-blocking read in MPSC/MPMC channels.
    #[error("channel error: {0}")]
    TryRecv(mpsc::TryRecvError),
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

impl From<mavio::errors::Error> for Error {
    fn from(value: mavio::errors::Error) -> Self {
        match value {
            mavio::errors::Error::Io(err) => Self::Io(err),
            mavio::errors::Error::Frame(err) => Self::Frame(err),
            mavio::errors::Error::Spec(err) => Self::Spec(err),
            #[allow(deprecated)]
            mavio::errors::Error::Buffer(err) => Self::Other(format!("{err:?}")),
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

impl<T> From<mpsc::SendError<T>> for Error {
    fn from(value: mpsc::SendError<T>) -> Self {
        SyncError::ChannelClosed(format!("MPSC send: {value:?}")).into()
    }
}

impl From<mpsc::TryRecvError> for Error {
    fn from(value: mpsc::TryRecvError) -> Self {
        SyncError::TryRecv(value).into()
    }
}

impl From<mpsc::RecvError> for Error {
    fn from(value: mpsc::RecvError) -> Self {
        SyncError::ChannelClosed(format!("MPSC recv: {value:?}")).into()
    }
}
