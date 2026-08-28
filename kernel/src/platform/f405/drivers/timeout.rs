//! Bounded polling policy shared by F405 peripheral adapters.

use dali_driver_api::{BoundedTimeout, DriverError, DriverResult, Duration};

/// Converts contract timeout ticks into a finite peripheral poll budget.
#[derive(Clone, Copy)]
pub(crate) struct F405TimeoutConfig {
    maximum_timeout: Duration,
    polls_per_tick: u32,
}

impl F405TimeoutConfig {
    /// Creates a timeout policy from target-owned timing metadata.
    pub(crate) const fn new(maximum_timeout: Duration, polls_per_tick: u32) -> Self {
        Self {
            maximum_timeout,
            polls_per_tick,
        }
    }

    /// Validates a contract timeout and returns its finite polling budget.
    pub(crate) fn poll_budget(&self, timeout: Duration) -> DriverResult<u32> {
        self.validate_timeout(timeout)?;
        timeout
            .ticks()
            .checked_mul(self.polls_per_tick)
            .filter(|budget| *budget != 0)
            .ok_or(DriverError::Timeout)
    }
}

impl BoundedTimeout for F405TimeoutConfig {
    fn max_timeout(&self) -> Duration {
        self.maximum_timeout
    }
}
