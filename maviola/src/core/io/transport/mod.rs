//! # 🔒 Transport interfaces

mod file;
mod serial;
#[cfg(unix)]
mod sock;
mod tcp;
mod udp;

pub use file::reader::FileReader;
pub use file::writer::FileWriter;
pub use tcp::client::TcpClient;
pub use tcp::server::TcpServer;
pub use udp::client::UdpClient;
pub use udp::server::UdpServer;

#[cfg(unix)]
pub use sock::client::SockClient;
#[cfg(unix)]
pub use sock::server::SockServer;

pub use serial::serial::SerialPort;
