//! # Maviola prelude
//!
//! This module contains basic imports for Maviola.

pub use crate::core::error::{Error, Result, SendResult};
pub use crate::core::error::{FrameError, NodeError, SyncError};
pub use crate::core::network::Network;
pub use crate::core::node::{CallbackApi, Node};
pub use crate::dialects::Minimal;
pub use crate::protocol::{
    CompatProcessor, CompatStrategy, Dialect, Endpoint, Frame, FrameSigner, MavLinkId,
    MavLinkVersion, MaybeVersioned, Message, SignStrategy, Versioned, Versionless, V1, V2,
};

pub use crate::core::io::{FileReader, FileWriter, TcpClient, TcpServer, UdpClient, UdpServer};
#[cfg(unix)]
pub use crate::core::io::{SockClient, SockServer};

#[cfg(feature = "unsafe")]
pub use crate::core::utils::TryUpdateFrom;
