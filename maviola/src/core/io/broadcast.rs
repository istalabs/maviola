use std::sync::Arc;

use crate::core::utils::UniqueId;
use crate::protocol::{Frame, MaybeVersioned};

/// Opaque type that contains an outgoing MAVLink [`Frame`].
#[derive(Clone, Debug)]
pub struct OutgoingFrame<V: MaybeVersioned> {
    frame: Arc<Frame<V>>,
    scope: BroadcastScope,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub(crate) enum BroadcastScope {
    #[default]
    All,
    Except(UniqueId),
    Exact(UniqueId),
}

impl<V: MaybeVersioned> OutgoingFrame<V> {
    /// Default constructor.
    pub fn new(frame: Frame<V>) -> Self {
        Self {
            frame: Arc::new(frame),
            scope: BroadcastScope::All,
        }
    }

    /// Reference to the underlying MAVLink [`Frame`].
    #[inline]
    pub fn frame(&self) -> &Frame<V> {
        self.frame.as_ref()
    }

    #[inline]
    pub(crate) fn scoped(frame: Frame<V>, scope: BroadcastScope) -> Self {
        Self {
            frame: Arc::new(frame),
            scope,
        }
    }

    pub(crate) fn should_send_to(&self, recipient_id: UniqueId) -> bool {
        match self.scope {
            BroadcastScope::All => true,
            BroadcastScope::Except(sender_id) => sender_id != recipient_id,
            BroadcastScope::Exact(sender_id) => sender_id == recipient_id,
        }
    }
}
