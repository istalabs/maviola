use std::cmp::Ordering;
use std::fmt::{Debug, Formatter};
use std::time::SystemTime;

use crate::protocol::{ComponentId, MavLinkId, SystemId};

/// MAVLink device with [`system_id`](Peer::system_id) and [`component_id`](Peer::component_id).
#[derive(Clone, Eq)]
pub struct Peer {
    pub(crate) id: MavLinkId,
    pub(crate) last_active: SystemTime,
}

impl Peer {
    /// Creates a new Peer with specified `system_id` and `component_id`.
    pub fn new(system_id: SystemId, component_id: ComponentId) -> Self {
        Self {
            id: MavLinkId {
                system: system_id,
                component: component_id,
            },
            last_active: SystemTime::now(),
        }
    }

    /// MAVLink system `ID`.
    #[inline]
    pub fn system_id(&self) -> SystemId {
        self.id.system
    }

    /// MAVLink component `ID`.
    #[inline]
    pub fn component_id(&self) -> ComponentId {
        self.id.component
    }

    /// Time when this peer sent the last message.
    #[inline]
    pub fn last_active(&self) -> SystemTime {
        self.last_active
    }
}

impl PartialEq for Peer {
    /// Two peers are considered equal if they have the same [`Peer::system_id`] and
    /// [`Peer::component_id`].
    fn eq(&self, other: &Self) -> bool {
        self.id.eq(&other.id)
    }
}

impl PartialOrd for Peer {
    /// Two can be compared if they have the same [`Peer::system_id`] and
    /// [`Peer::component_id`]. If so, then [`Peer::last_active`] will be compared.
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self != other {
            None
        } else {
            self.last_active.partial_cmp(&other.last_active)
        }
    }
}

impl Debug for Peer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Peer")
            .field("system_id", &self.system_id())
            .field("component_id", &self.component_id())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod peer_tests {
    use super::*;

    use std::time::UNIX_EPOCH;

    #[test]
    fn peer_comparisons() {
        let peer_1_old = Peer {
            id: MavLinkId {
                system: 42,
                component: 17,
            },
            last_active: UNIX_EPOCH,
        };

        let peer_1_new = Peer {
            id: MavLinkId {
                system: 42,
                component: 17,
            },
            last_active: SystemTime::now(),
        };

        let peer_2_old = Peer {
            id: MavLinkId {
                system: 1,
                component: 10,
            },
            last_active: UNIX_EPOCH,
        };

        let peer_2_new = Peer {
            id: MavLinkId {
                system: 1,
                component: 10,
            },
            last_active: peer_1_new.last_active,
        };

        assert_eq!(peer_1_new, peer_1_old);
        assert_eq!(peer_2_new, peer_2_old);
        assert_ne!(peer_1_new, peer_2_new);

        assert!(peer_1_old < peer_1_new);
        assert!(peer_1_old <= peer_1_new);

        assert!(!(peer_1_old < peer_2_new));
    }
}
