//! Maviola test utils.
//!
//! These utils are generated only when `#[cfg(test)]` is enabled.

use std::thread;
use std::time::Duration;

pub const WAIT_DURATION: Duration = Duration::from_micros(100);
pub const WAIT_LONG_DURATION: Duration = Duration::from_micros(1000);

pub fn wait() {
    thread::sleep(WAIT_DURATION)
}

pub fn wait_long() {
    thread::sleep(WAIT_LONG_DURATION)
}
