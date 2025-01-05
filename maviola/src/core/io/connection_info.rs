use std::fmt::{Debug, Formatter};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Once;

use crate::core::io::{ChannelId, ConnectionId};

/// <sup>[`serde`](https://serde.rs) | [`specta`](https://crates.io/crates/specta)</sup>
/// Information about a connection.
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone)]
pub struct ConnectionInfo {
    id: ConnectionId,
    details: ConnectionDetails,
}

/// <sup>[`serde`](https://serde.rs) | [`specta`](https://crates.io/crates/specta)</sup>
/// Information about a connection.
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub enum ConnectionDetails {
    /// TCP server.
    TcpServer {
        /// Server address.
        bind_addr: SocketAddr,
    },
    /// TCP client.
    TcpClient {
        /// Server address.
        remote_addr: SocketAddr,
    },
    /// UDP server.
    UdpServer {
        /// Server address.
        bind_addr: SocketAddr,
    },
    /// TCP client.
    UdpClient {
        /// Server address.
        remote_addr: SocketAddr,
    },
    /// Writes binary output to a file.
    FileWriter {
        /// File path.
        path: PathBuf,
    },
    /// Reads binary output from a file.
    FileReader {
        /// File path.
        path: PathBuf,
    },
    /// <sup>`unix`</sup>
    /// Unix socket server.
    #[cfg(unix)]
    SockServer {
        /// Socket path.
        path: PathBuf,
    },
    /// <sup>`unix`</sup>
    /// Unix socket client.
    #[cfg(unix)]
    SockClient {
        /// Server address.
        path: PathBuf,
    },
    /// Serial port.
    SerialPort {
        /// Port path.
        path: String,
        /// Baud rate.
        baud_rate: u32,
    },
    /// Network with multiple connections.
    Network,
    /// Custom connection.
    #[cfg(feature = "unstable")]
    Custom {
        /// Name of the custom connection.
        name: String,
        /// Implementation-specific details.
        details: String,
    },
    /// Unknown connection.
    Unknown,
}

/// Information about a channel within a particular connection.
#[derive(Clone)]
pub struct ChannelInfo {
    id: ChannelId,
    details: ChannelDetails,
}

/// Information about a channel within a particular connection.
///
/// A particular connection may have several channels. For example, a TCP server creates a separate
/// stream for each client.
#[derive(Clone, Debug)]
pub enum ChannelDetails {
    /// TCP server.
    TcpServer {
        /// Server address.
        server_addr: SocketAddr,
        /// Peer address.
        peer_addr: SocketAddr,
    },
    /// TCP client.
    TcpClient {
        /// Server address.
        server_addr: SocketAddr,
    },
    /// UDP server.
    UdpServer {
        /// Server address.
        server_addr: SocketAddr,
        /// Peer address.
        peer_addr: SocketAddr,
    },
    /// UDP client.
    UdpClient {
        /// Remote server address.
        server_addr: SocketAddr,
        /// Bind address.
        bind_addr: SocketAddr,
    },
    /// Writes binary output to a file.
    FileWriter {
        /// File path.
        path: PathBuf,
    },
    /// Reads binary output from a file.
    FileReader {
        /// File path.
        path: PathBuf,
    },
    /// <sup>`unix`</sup>
    /// Unix socket server.
    #[cfg(unix)]
    SockServer {
        /// Socket path.
        path: PathBuf,
    },
    /// <sup>`unix`</sup>
    /// Unix socket client.
    #[cfg(unix)]
    SockClient {
        /// Socket path.
        path: PathBuf,
    },
    /// Serial port.
    SerialPort {
        /// Path to port.
        path: String,
        /// Baud rate.
        baud_rate: u32,
    },
    /// Custom channel.
    #[cfg(feature = "unstable")]
    Custom {
        /// Name of the custom connection.
        conn_name: String,
        /// Name of the custom channel.
        channel_name: String,
        /// Implementation-specific details.
        details: String,
    },
    /// Unknown channel
    Unknown,
}

impl ConnectionInfo {
    /// Creates a new instance of [`ConnectionInfo`].
    #[cfg(feature = "unstable")]
    #[inline(always)]
    pub fn new(details: ConnectionDetails) -> Self {
        Self::new_inner(details)
    }

    #[cfg(not(feature = "unstable"))]
    #[inline(always)]
    pub(crate) fn new(details: ConnectionDetails) -> Self {
        Self::new_inner(details)
    }

    /// Connection `ID`.
    pub fn id(&self) -> ConnectionId {
        self.id
    }

    /// Connection details.
    pub fn details(&self) -> &ConnectionDetails {
        &self.details
    }

    /// Creates [`ChannelInfo`] for a channel withing this connection.
    #[cfg(feature = "unstable")]
    #[inline(always)]
    pub fn make_channel_info(&self, details: ChannelDetails) -> ChannelInfo {
        self.make_channel_info_inner(details)
    }

    /// Creates [`ChannelInfo`] for a channel withing this connection.
    #[cfg(not(feature = "unstable"))]
    #[inline(always)]
    pub fn make_channel_info(&self, details: ChannelDetails) -> ChannelInfo {
        self.make_channel_info_inner(details)
    }

    fn new_inner(details: ConnectionDetails) -> Self {
        Self {
            id: ConnectionId::new(),
            details,
        }
    }

    fn make_channel_info_inner(&self, details: ChannelDetails) -> ChannelInfo {
        ChannelInfo::new(self.id, details)
    }
}

impl Debug for ConnectionInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.details.fmt(f)
    }
}

impl ChannelInfo {
    /// Creates a new instance of [`ChannelInfo`].
    #[cfg(feature = "unstable")]
    #[inline(always)]
    pub fn new(connection_id: ConnectionId, details: ChannelDetails) -> Self {
        Self::new_inner(connection_id, details)
    }

    #[cfg(not(feature = "unstable"))]
    #[inline(always)]
    pub(crate) fn new(connection_id: ConnectionId, details: ChannelDetails) -> Self {
        Self::new_inner(connection_id, details)
    }

    /// Channel `ID`.
    pub fn id(&self) -> ChannelId {
        self.id
    }

    /// Connection `ID` of this channel.
    pub fn connection_id(&self) -> ConnectionId {
        self.id.connection_id()
    }

    /// Channel details.
    pub fn details(&self) -> &ChannelDetails {
        &self.details
    }

    fn new_inner(connection_id: ConnectionId, details: ChannelDetails) -> Self {
        Self {
            id: ChannelId::new(connection_id),
            details,
        }
    }
}

impl Debug for ChannelInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.details.fmt(f)
    }
}

static mut UNKNOWN_CONNECTION: Option<ConnectionInfo> = None;
static INIT_UNKNOWN_CONNECTION: Once = Once::new();

impl ConnectionInfo {
    #[allow(static_mut_refs)]
    pub(in crate::core) fn unknown() -> &'static ConnectionInfo {
        INIT_UNKNOWN_CONNECTION.call_once(|| unsafe {
            UNKNOWN_CONNECTION = Some(ConnectionInfo::new(ConnectionDetails::Unknown));
        });
        unsafe { UNKNOWN_CONNECTION.as_ref().unwrap() }
    }
}
