use std::fmt::Debug;

use crate::core::io::ConnectionInfo;

/// Connection configuration for a [`Node`](crate::core::node::Node).
pub trait ConnectionConf: Debug + Send {
    /// Provides information about connection.
    fn info(&self) -> &ConnectionInfo;
}
