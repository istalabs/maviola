//! Maviola test utils.
//!
//! These utils are generated only when `#[cfg(test)]` is enabled.

use std::sync::Once;

const LOG_LEVEL: log::LevelFilter = log::LevelFilter::Debug;

static INIT: Once = Once::new();
static INIT_LOGGER: Once = Once::new();

pub fn init_logger() {
    INIT_LOGGER.call_once(|| {
        env_logger::builder()
            // Suppress everything below `warn` for third-party modules
            .filter_level(log::LevelFilter::Warn)
            // Allow everything above `LOG_LEVEL` from current package
            .filter_module(env!("CARGO_PKG_NAME"), LOG_LEVEL)
            .init();
    });
}

pub fn initialize() {
    INIT.call_once(|| init_logger());
}
