//! Maviola synchronous I/O.

pub(crate) mod connection;
pub(crate) mod response;
mod tcp;

pub use connection::{Connection, ConnectionInfo};
pub use response::Response;
pub use tcp::client::TcpClientConf;
pub use tcp::server::TcpServerConf;
