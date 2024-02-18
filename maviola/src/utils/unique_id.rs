use std::cmp::Ordering;
use std::fmt::{Debug, Formatter};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// Unique identifier.
///
/// Identifier which is guaranteed to be unique during the program run. It is intentionally kept
/// opaque. This identifier can't be serialized or deserialized and dedicated for comparison of
/// runtime entities like nodes or connections.
#[derive(Copy, Clone, Eq, Ord, Hash)]
pub struct UniqueId {
    timestamp: u64,
    counter: UniqueIdCounter,
}

static UNIQUE_ID: Mutex<UniqueId> = Mutex::new(UniqueId {
    timestamp: 0,
    counter: 0,
});

type UniqueIdCounter = u16;

impl Debug for UniqueId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("UniqueId")
            .field(&self.timestamp)
            .field(&self.counter)
            .finish()
    }
}

impl PartialEq for UniqueId {
    fn eq(&self, other: &Self) -> bool {
        self.timestamp == other.timestamp && self.counter == other.counter
    }
}

impl PartialOrd for UniqueId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let timestamp_cmp = self.timestamp.cmp(&other.timestamp);
        if timestamp_cmp != Ordering::Equal {
            return Some(timestamp_cmp);
        }

        Some(self.counter.cmp(&other.counter))
    }
}

impl UniqueId {
    /// Generates unique identifier.
    pub fn new() -> Self {
        let mut id = UNIQUE_ID.lock().unwrap();
        let (mut timestamp, counter) = (id.timestamp, id.counter);

        if counter == 0 {
            let new_timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            if new_timestamp > timestamp {
                timestamp = new_timestamp;
            } else {
                timestamp += 1;
            }
        }

        let next_counter = match counter {
            UniqueIdCounter::MAX => 0,
            current => current + 1,
        };

        id.timestamp = timestamp;
        id.counter = next_counter;

        Self { timestamp, counter }
    }
}

#[cfg(test)]
mod unique_id_tests {
    use super::*;

    #[test]
    fn test_unique_id() {
        let id_0 = UniqueId::new();
        let id_1 = UniqueId::new();

        assert_eq!(id_0, id_0);
        assert!(id_0 < id_1);
        assert_ne!(id_0, id_1);

        {
            let mut id = UNIQUE_ID.lock().unwrap();
            id.counter = UniqueIdCounter::MAX;
        }

        let id_0 = UniqueId::new();
        let id_1 = UniqueId::new();

        assert_eq!(id_0, id_0);
        assert!(id_0 < id_1);
        assert_ne!(id_0, id_1);
    }
}
