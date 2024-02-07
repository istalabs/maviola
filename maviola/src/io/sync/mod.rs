//! Maviola synchronous I/O.

pub mod connection;
pub mod tcp;

#[doc(inline)]
pub use connection::{Connection, ConnectionConf};
#[doc(inline)]
pub use tcp::client::TcpClientConf;
#[doc(inline)]
pub use tcp::server::TcpServerConf;
