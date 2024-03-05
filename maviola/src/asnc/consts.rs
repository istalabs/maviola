use std::time::Duration;

pub(crate) const CONN_BROADCAST_CHAN_CAPACITY: usize = 1024 * 32;

pub(crate) const CHANNEL_STOP_POOLING_INTERVAL: Duration = Duration::from_micros(100);
pub(crate) const CHANNEL_STOP_JOIN_POOLING_INTERVAL: Duration = Duration::from_millis(100);
pub(crate) const CHANNEL_STOP_JOIN_ATTEMPTS: usize = 30;

pub(crate) const EVENTS_RECV_POOLING_INTERVAL: Duration = Duration::from_millis(1);

pub(crate) const CONN_STOP_POOLING_INTERVAL: Duration = Duration::from_millis(10);
pub(crate) const TCP_READ_TIMEOUT: Option<Duration> = None;
pub(crate) const TCP_WRITE_TIMEOUT: Option<Duration> = None;

pub(crate) const UDP_RETRIES: usize = 5;
pub(crate) const UDP_RETRY_INTERVAL: Duration = Duration::from_millis(20);

#[cfg(unix)]
pub(crate) const SOCK_ACCEPT_INTERVAL: Duration = Duration::from_millis(100);
#[cfg(unix)]
pub(crate) const SOCK_READ_TIMEOUT: Option<Duration> = Some(Duration::from_millis(500));
#[cfg(unix)]
pub(crate) const SOCK_WRITE_TIMEOUT: Option<Duration> = Some(Duration::from_micros(50));
