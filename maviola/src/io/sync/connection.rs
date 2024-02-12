//! Maviola synchronous connection and its configuration.

use mavio::protocol::MaybeVersioned;
use std::fmt::Debug;
use std::net::SocketAddr;
use std::sync::{mpsc, Arc, Mutex};

use crate::prelude::*;

/// Connection events.
pub enum ConnectionEvent<V: MaybeVersioned> {
    /// New connection was established.
    New(Box<dyn Connection<V>>),
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
pub trait Connection<V: MaybeVersioned>: Send + Debug {
    /// Connection ID.
    fn id(&self) -> usize;

    /// Information about current configuration.
    fn info(&self) -> &ConnectionInfo;

    /// Receive MAVLink frame.
    fn receiver(&self) -> Arc<Mutex<Box<dyn Receiver<V>>>>;

    /// Send MAVLink frame.
    fn sender(&self) -> Arc<Mutex<Box<dyn Sender<V>>>>;

    /// Close connection.
    fn close(&self) -> Result<()>;
}

/// MAVLink [`Frame`] sender.
pub trait Sender<V: MaybeVersioned>: Send + Sync + Debug {
    /// Send MAVLink frame.
    fn send(&mut self, frame: &mavio::Frame<V>) -> Result<usize>;
}

/// MAVLink [`Frame`] receiver.
pub trait Receiver<V: MaybeVersioned>: Send + Sync + Debug {
    /// Receive MAVLink frame.
    fn recv(&mut self) -> Result<mavio::Frame<V>>;
}

/// Connection builder used to create a [`Connection`].
pub trait ConnectionBuilder<V: MaybeVersioned>: Debug + Send {
    /// Builds [`Connection`] from provided configuration.
    fn build(&self) -> Result<mpsc::Receiver<ConnectionEvent<V>>>;
}

/// Connection configuration.
pub trait ConnectionConf<V: MaybeVersioned>: ConnectionBuilder<V> {
    /// Information about connection config.
    fn info(&self) -> ConnectionConfInfo;
}
