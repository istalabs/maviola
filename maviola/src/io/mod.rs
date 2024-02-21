//! # Maviola I/O

#[cfg(feature = "sync")]
pub mod asnc;
mod broadcast;
mod connection_info;
pub mod marker;
mod node_builder;
mod node_conf;
#[cfg(feature = "sync")]
pub mod sync;
mod utils;

pub use broadcast::OutgoingFrame;
pub use connection_info::{ConnectionInfo, PeerConnectionInfo};
pub use node_builder::NodeBuilder;
pub use node_conf::NodeConf;

#[doc(inline)]
#[cfg(feature = "sync")]
/// <sup>[`sync`]</sup>
pub use sync::{Callback, Connection, Event, Node};
#[doc(inline)]
#[cfg(feature = "sync")]
/// <sup>[`sync`]</sup>
pub use sync::{FileReader, FileWriter, TcpClient, TcpServer, UdpClient, UdpServer};
#[doc(inline)]
#[cfg(feature = "sync")]
/// <sup>[`sync`] |</sup>
pub use sync::{SockClient, SockServer};

#[doc(inline)]
#[cfg(feature = "sync")]
/// <sup>[`sync`]</sup>
/// <sup>| [`mavio`](https://docs.rs/mavio/0.2.0-rc2/mavio/)</sup>
pub use sync::{Receiver, Sender};

#[doc(inline)]
#[cfg(feature = "async")]
/// <sup>[`async`](asnc)</sup>
pub use asnc::{AsyncConnection, AsyncResponse};

#[doc(inline)]
#[cfg(feature = "async")]
/// <sup>[`async`](asnc)</sup>
/// <sup>| [`mavio`](https://docs.rs/mavio/0.2.0-rc2/mavio/)</sup>
pub use asnc::{AsyncReceiver, AsyncSender};
