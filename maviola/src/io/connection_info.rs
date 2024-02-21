use std::net::SocketAddr;
use std::path::PathBuf;

/// Information about a connection.
#[derive(Clone, Debug)]
pub enum ConnectionInfo {
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
    /// Custom connection.
    Custom {
        /// Name of the custom connection.
        name: String,
        /// Implementation-specific details.
        details: String,
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
}

/// Information about a channel within a particular connection.
///
/// A particular connection may have several channels. For example, a TCP server creates a separate
/// stream for each client.
#[derive(Clone, Debug)]
pub enum ChannelInfo {
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
    /// Custom channel.
    Custom {
        /// Name of the custom connection.
        conn_name: String,
        /// Name of the custom channel.
        channel_name: String,
        /// Implementation-specific details.
        details: String,
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
}
