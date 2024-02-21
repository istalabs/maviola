use std::time::Duration;

pub(super) const PEER_CONN_STOP_POOLING_INTERVAL: Duration = Duration::from_micros(100);
pub(super) const PEER_CONN_STOP_JOIN_POOLING_INTERVAL: Duration = Duration::from_millis(100);
pub(super) const PEER_CONN_STOP_JOIN_ATTEMPTS: usize = 30;

pub(super) const CONN_STOP_POOLING_INTERVAL: Duration = Duration::from_millis(10);
pub(super) const TCP_READ_TIMEOUT: Option<Duration> = None;
pub(super) const TCP_WRITE_TIMEOUT: Option<Duration> = None;

pub(super) const UDP_RETRIES: usize = 5;
pub(super) const UDP_RETRY_INTERVAL: Duration = Duration::from_millis(20);

#[cfg(unix)]
pub(super) const SOCK_ACCEPT_INTERVAL: Duration = Duration::from_millis(100);
#[cfg(unix)]
pub(super) const SOCK_READ_TIMEOUT: Option<Duration> = Some(Duration::from_millis(500));
#[cfg(unix)]
pub(super) const SOCK_WRITE_TIMEOUT: Option<Duration> = Some(Duration::from_micros(50));
