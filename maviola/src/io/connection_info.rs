use std::net::SocketAddr;

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
}

/// Information about a peer connection.
///
/// A particular connection may have several peer connection. For example, a TCP server creates
/// a peer connection for each client.
#[derive(Clone, Debug)]
pub enum PeerConnectionInfo {
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
}
