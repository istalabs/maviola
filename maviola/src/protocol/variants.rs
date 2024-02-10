//! Common markers for generic entities.

use std::fmt::{Debug, Formatter};

use mavio::protocol::{DialectImpl, DialectMessage, MavLinkVersion};

use crate::prelude::*;

use crate::protocol::CoreFrame;

/// Marks structures which may or may not have MAVLink protocol version.
pub trait IsVersioned: Clone {
    /// Validates that provided frame matches MAVLink protocol version.
    fn matches_frame(&self, _: &CoreFrame) -> Result<()> {
        Ok(())
    }
}

/// Marker for entities which do not have a specific MAVLink protocol version.
#[derive(Clone, Debug, Default)]
pub struct NotVersioned();
impl IsVersioned for NotVersioned {}

/// Marks entities which have a specified MAVLink protocol version.
pub trait Versioned: IsVersioned {
    /// MAVLink protocol version.
    fn mavlink_version(&self) -> MavLinkVersion;

    /// Validates that provided frame matches MAVLink protocol version.
    fn matches_frame(&self, frame: &CoreFrame) -> Result<()> {
        if self.mavlink_version() != frame.mavlink_version() {
            return Err(FrameBuildError::InvalidVersion {
                frame: frame.clone(),
                given: frame.mavlink_version(),
                expected: self.mavlink_version(),
            }
            .into());
        }
        Ok(())
    }
}

/// Marks entities which are only `MAVLink 1` protocol compliant.
#[derive(Clone, Copy, Debug, Default)]
pub struct MavLink1();
impl IsVersioned for MavLink1 {}
impl Versioned for MavLink1 {
    fn mavlink_version(&self) -> MavLinkVersion {
        MavLinkVersion::V1
    }
}

/// Marks entities which are only `MAVLink 2` protocol compliant.
#[derive(Clone, Copy, Debug, Default)]
pub struct MavLink2();
impl IsVersioned for MavLink2 {}
impl Versioned for MavLink2 {
    fn mavlink_version(&self) -> MavLinkVersion {
        MavLinkVersion::V2
    }
}

/// Marker for entities which depend on whether a particular dialect has been specified..
pub trait IsDialect: Clone + Debug {
    /// Validates that provided frame exists in the dialect.
    fn matches_frame(&self, _: &CoreFrame) -> Result<()> {
        Ok(())
    }
}

/// Marks entities without a specific MAVLink dialect.
#[derive(Clone, Copy, Debug)]
pub struct NoDialect();
impl IsDialect for NoDialect {}

/// Provides a reference to dialect and marks structures which depend on a dialect being specified.
pub struct HasDialect<M: DialectMessage + 'static>(
    pub(crate) &'static dyn DialectImpl<Message = M>,
);
impl<M: DialectMessage + 'static> Clone for HasDialect<M> {
    fn clone(&self) -> Self {
        HasDialect(self.0)
    }
}
impl<M: DialectMessage + 'static> Debug for HasDialect<M> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let dialect = f
            .debug_struct("Dialect")
            .field("name", &self.0.name())
            .finish_non_exhaustive();
        f.debug_struct("HasDialect").field("0", &dialect).finish()
    }
}
impl<M: DialectMessage + 'static> IsDialect for HasDialect<M> {
    fn matches_frame(&self, frame: &CoreFrame) -> Result<()> {
        if self.0.message_info(frame.message_id()).is_err() {
            return Err(FrameBuildError::NotInDialect(
                frame.clone(),
                frame.message_id(),
                self.0.name(),
            )
            .into());
        }
        Ok(())
    }
}
