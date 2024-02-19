//! # Maviola I/O

mod connection_info;
mod node_conf;
#[cfg(feature = "sync")]
pub mod sync;
mod utils;

pub use connection_info::{ConnectionInfo, PeerConnectionInfo};
pub use node_conf::builder::NodeConfBuilder;
pub use node_conf::NodeConf;
#[doc(inline)]
#[cfg(feature = "sync")]
/// <sup>[`sync`]</sup>
pub use sync::{Connection, Event, Node, Response};
#[doc(inline)]
#[cfg(feature = "sync")]
/// <sup>[`sync`]</sup>
pub use sync::{TcpClientConf, TcpServerConf, UdpClientConf, UdpServerConf};

#[doc(inline)]
#[cfg(feature = "sync")]
/// <sup>[`sync`]</sup>
/// <sup>| From [`mavio`](https://docs.rs/mavio/0.2.0-rc2/mavio/)</sup>
pub use sync::{Receiver, Sender};
