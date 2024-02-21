use core::fmt::{Debug, Formatter};

use crate::protocol::{DialectImpl, DialectMessage, MessageId};
use crate::utils::Sealed;

use crate::prelude::*;

/// <sup>🔒</sup>
/// Marker for entities which depend on whether a particular dialect has been specified.
///
/// ⚠ This trait is sealed ⚠
///
/// Entities which implement this dialect expose [`MaybeDialect::matches`] and
/// [`MaybeDialect::expect`] methods to validate if particular message `ID` belongs to associated
/// dialect. The blanket implementations assume no dialect and accept any message by
/// [vacuous truth](https://en.wikipedia.org/wiki/Vacuous_truth).
pub trait MaybeDialect: Clone + Debug + Sync + Send + Sealed {
    /// Checks that provided message `ID` exists in the dialect.
    ///
    /// The blanket implementation always return `true` which means that, by default, everything is
    /// compatible.
    #[inline]
    fn matches(&self, #[allow(unused_variables)] message_id: MessageId) -> bool {
        true
    }

    /// Validates that provided message `ID` exists in the dialect.
    ///
    /// Throws [NodeError::NotInDialect] if validation failed.
    ///
    /// The blanket implementation always return [`Ok`] which means that, by default, everything is
    /// compatible.
    #[inline]
    fn expect(&self, #[allow(unused_variables)] message_id: MessageId) -> Result<()> {
        Ok(())
    }
}

/// Marks entities without a specific MAVLink dialect.
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Dialectless;
impl Sealed for Dialectless {}
impl MaybeDialect for Dialectless {}

/// Provides a reference to dialect and marks structures which depend on a dialect being specified.
pub struct HasDialect<M: DialectMessage + 'static>(pub &'static dyn DialectImpl<Message = M>);
impl<M: DialectMessage + 'static> Sealed for HasDialect<M> {}
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
    #[inline]
    fn matches(&self, message_id: MessageId) -> bool {
        self.0.message_info(message_id).is_err()
    }

    #[inline]
    fn expect(&self, message_id: MessageId) -> Result<()> {
        if self.0.message_info(message_id).is_err() {
            return Err(NodeError::NotInDialect(message_id, self.0.name()).into());
        }
        Ok(())
    }
}
