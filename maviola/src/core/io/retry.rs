use std::time::Duration;

/// Defines a retry strategy.
///
/// When an entity goes down and can be repaired, then it may be rebuilt/restored according to this
/// strategy.
#[derive(Copy, Clone, Debug, Default)]
pub enum Retry {
    /// Never restore (default value).
    #[default]
    Never,
    /// Always retry to restore with a specified interval.
    Always(
        /// Interval between attempts.
        Duration,
    ),
    /// Perform several restore attempts with a specified interval.
    Attempts(
        /// Number of attempts.
        usize,
        /// Interval between attempts.
        Duration,
    ),
}
