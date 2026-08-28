//! Timer lifecycle contract.

use super::{BoundedTimeout, Duration};
use crate::DriverResult;

/// Basic timer lifecycle independent of interrupt delivery.
pub trait TimerDriver: BoundedTimeout {
    /// Starts or restarts the timer.
    fn start(&mut self, timeout: Duration) -> DriverResult<()>;

    /// Stops the timer and releases its active state.
    fn stop(&mut self) -> DriverResult<()>;

    /// Returns whether the timer is currently active.
    fn is_running(&self) -> DriverResult<bool>;

    /// Returns whether the active timer has expired.
    fn is_expired(&mut self) -> DriverResult<bool>;
}
