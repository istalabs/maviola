use crate::core::consts::SERIAL_CONN_TIMEOUT;
use crate::core::io::{ConnectionConf, ConnectionDetails, ConnectionInfo};

use crate::prelude::*;

/// Serial port client configuration.
///
/// Provides connection configuration for a node that connects to a serial port.
///
/// # Usage
///
/// Create a synchronous node that connects to a port:
///
/// ```rust,no_run
/// # #[cfg(feature = "sync")] {
/// use maviola::prelude::*;
///
/// let path = "/dev/tty.usbmodem101";
/// let baud_rate = 115_200;
///
/// // Create a node that connects to a serial port
/// let node = Node::sync::<V2>()
///         /* define other node parameters */
/// #       .system_id(1)
/// #       .component_id(1)
///         .connection(
///             SerialPort::new(path, baud_rate)    // Configure serial port connection
///                 .unwrap()
///         ).build().unwrap();
/// # }
/// ```
///
/// Create an asynchronous node that reads from a file:
///
/// ```rust,no_run
/// # #[cfg(not(feature = "async"))] fn main() {}
/// # #[cfg(feature = "async")]
/// # #[tokio::main] async fn main() {
/// use maviola::prelude::*;
///
/// let path = "/dev/tty.usbmodem101";
/// let baud_rate = 115_200;
///
/// // Create a node that connects to a serial port
/// let node = Node::asnc::<V2>()
///         /* define other node parameters */
/// #       .system_id(1)
/// #       .component_id(1)
///         .connection(
///             SerialPort::new(path, baud_rate)    // Configure serial port connection
///                 .unwrap()
///         ).build().await.unwrap();
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct SerialPort {
    pub(crate) path: String,
    pub(crate) baud_rate: u32,
    pub(crate) conn_timeout: std::time::Duration,
    pub(crate) info: ConnectionInfo,
}

impl SerialPort {
    /// Instantiates a serial port configuration.
    ///
    /// Accepts as `path` anything that can be converted to a [String].
    /// Use `baud_rate` to set connection speed.
    pub fn new<'a>(path: impl Into<std::borrow::Cow<'a, str>>, baud_rate: u32) -> Result<Self> {
        let path = path.into().to_string();
        let conn_timeout = SERIAL_CONN_TIMEOUT;
        let info = ConnectionInfo::new(ConnectionDetails::SerialPort {
            path: path.clone(),
            baud_rate,
        });
        Ok(Self {
            path,
            baud_rate,
            conn_timeout,
            info,
        })
    }
}

impl ConnectionConf for SerialPort {
    fn info(&self) -> &ConnectionInfo {
        &self.info
    }
}
