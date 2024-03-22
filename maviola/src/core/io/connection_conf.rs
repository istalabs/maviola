use std::fmt::Debug;

use crate::core::io::{ConnectionId, ConnectionInfo};

/// Connection configuration for a [`Node`](crate::core::node::Node).
pub trait ConnectionConf: Debug + Send {
    /// Provides information about connection.
    fn info(&self) -> &ConnectionInfo;

    /// Returns connection identifier.
    ///
    /// We suggest not to reimplement this method unless you are really know what you are doing.
    fn id(&self) -> ConnectionId {
        self.info().id()
    }
}
