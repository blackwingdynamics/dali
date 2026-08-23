//! Bounded countdown contracts.

use crate::{DriverError, DriverResult};

/// A target-independent duration represented by adapter-defined ticks.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Duration(u32);

impl Duration {
    /// Creates a duration from an adapter-defined tick count.
    pub const fn from_ticks(ticks: u32) -> Self {
        Self(ticks)
    }

    /// Returns the adapter-defined tick count.
    pub const fn ticks(self) -> u32 {
        self.0
    }
}

/// Declares and validates the timeout range supported by a timer.
pub trait BoundedTimeout {
    /// Returns the largest accepted timeout.
    fn max_timeout(&self) -> Duration;

    /// Rejects zero, overflowing, or otherwise unsupported durations.
    fn validate_timeout(&self, timeout: Duration) -> DriverResult<()> {
        if timeout.ticks() == 0 || timeout.ticks() > self.max_timeout().ticks() {
            return Err(DriverError::Timeout);
        }
        Ok(())
    }
}

/// A one-shot countdown with bounded wait and cancellation operations.
pub trait CountDown: BoundedTimeout {
    /// Starts a countdown for the supplied validated duration.
    fn start(&mut self, timeout: Duration) -> DriverResult<()>;

    /// Observes whether the countdown has expired.
    fn wait(&mut self) -> DriverResult<bool>;

    /// Cancels an active countdown.
    fn cancel(&mut self) -> DriverResult<()>;
}
