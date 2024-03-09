use std::time::Duration;

pub(crate) const CONN_BROADCAST_CHAN_CAPACITY: usize = 1024 * 32;

pub(crate) const NETWORK_CLOSED_CHAN_CAPACITY: usize = 1024 * 32;
pub(crate) const NETWORK_RETRY_EVENTS_CHAN_CAPACITY: usize = 1024 * 32;

pub(crate) const CHANNEL_STOP_POOLING_INTERVAL: Duration = Duration::from_micros(100);
pub(crate) const CHANNEL_STOP_JOIN_POOLING_INTERVAL: Duration = Duration::from_millis(100);
pub(crate) const CHANNEL_STOP_JOIN_ATTEMPTS: usize = 30;

pub(crate) const EVENTS_RECV_POOLING_INTERVAL: Duration = Duration::from_millis(1);

pub(crate) const CONN_STOP_POOLING_INTERVAL: Duration = Duration::from_millis(10);
