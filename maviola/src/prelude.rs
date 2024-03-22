//! # Maviola prelude
//!
//! This module contains basic imports for Maviola.

pub use crate::core::consts::DefaultDialect;
pub use crate::core::node::{CallbackApi, Node, SendFrame, SendMessage, SendVersionlessMessage};
pub use crate::error::{Error, Result};
pub use crate::protocol::{
    CompatProcessor, CompatStrategy, Dialect, Endpoint, Frame, FrameSigner, MavLinkId,
    MavLinkVersion, MaybeVersioned, Message, SignStrategy, Versioned, Versionless, V1, V2,
};

pub use crate::core::io::{FileReader, FileWriter, TcpClient, TcpServer, UdpClient, UdpServer};
#[cfg(unix)]
pub use crate::core::io::{SockClient, SockServer};
pub use crate::core::network::Network;

#[cfg(feature = "unsafe")]
pub use crate::core::utils::TryUpdateFrom;
