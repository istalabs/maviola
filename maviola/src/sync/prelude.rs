//! # Common imports for synchronous API

pub use crate::sync::io::Callback;
pub use crate::sync::io::{FileReader, FileWriter, TcpClient, TcpServer, UdpClient, UdpServer};
pub use crate::sync::node::{EdgeNode, Event, ProxyNode, SyncApi};

#[cfg(unix)]
pub use crate::sync::io::{SockClient, SockServer};
