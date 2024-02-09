//! Maviola synchronous connection and its configuration.

use std::fmt::Debug;
use std::net::SocketAddr;
use std::sync::{mpsc, Arc, Mutex};

use mavio::Frame;

use crate::prelude::*;

/// Connection events.
pub enum ConnectionEvent {
    /// New connection was established.
    New(Box<dyn Connection>),
    /// Connection was dropped.
    Drop(usize, Option<Error>),
    /// Error during connection management.
    Error(Error),
}

/// Information about a connection configuration.
#[derive(Clone, Debug)]
pub enum ConnectionConfInfo {
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
}

/// Information about a connection.
#[derive(Clone, Debug)]
pub enum ConnectionInfo {
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
}

/// Synchronous connection.
pub trait Connection: Send + Debug {
    /// Connection ID.
    fn id(&self) -> usize;

    /// Information about current configuration.
    fn info(&self) -> &ConnectionInfo;

    /// Receive MAVLink frame.
    fn receiver(&self) -> Arc<Mutex<Box<dyn Receiver>>>;

    /// Send MAVLink frame.
    fn sender(&self) -> Arc<Mutex<Box<dyn Sender>>>;

    /// Close connection.
    fn close(&self) -> Result<()>;
}

/// MAVLink [`Frame`] sender.
pub trait Sender: Send + Sync + Debug {
    /// Send MAVLink frame.
    fn send(&mut self, frame: &Frame) -> Result<usize>;
}

/// MAVLink [`Frame`] receiver.
pub trait Receiver: Send + Sync + Debug {
    /// Receive MAVLink frame.
    fn recv(&mut self) -> Result<Frame>;
}

/// Connection builder used to create a [`Connection`].
pub trait ConnectionBuilder: Debug + Send {
    /// Builds [`Connection`] from provided configuration.
    fn build(&self) -> Result<mpsc::Receiver<ConnectionEvent>>;
}

/// Connection configuration.
pub trait ConnectionConf: ConnectionBuilder {
    /// Information about connection config.
    fn info(&self) -> ConnectionConfInfo;
}
