//! Common markers for generic entities.

use std::fmt::{Debug, Formatter};

use mavio::protocol::{DialectImpl, DialectMessage, MessageId};

use crate::prelude::*;

/// Marker for entities which depend on whether a particular dialect has been specified..
pub trait MaybeDialect: Clone + Debug {
    /// Validates that provided frame exists in the dialect.
    fn matches_frame(&self, _: MessageId) -> Result<()> {
        Ok(())
    }
}

/// Marks entities without a specific MAVLink dialect.
#[derive(Clone, Copy, Debug)]
pub struct Dialectless;
impl MaybeDialect for Dialectless {}

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
impl<M: DialectMessage + 'static> MaybeDialect for HasDialect<M> {
    fn matches_frame(&self, message_id: MessageId) -> Result<()> {
        if self.0.message_info(message_id).is_err() {
            return Err(FrameBuildError::NotInDialect(message_id, self.0.name()).into());
        }
        Ok(())
    }
}
