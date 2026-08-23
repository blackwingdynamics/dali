//! Serial transmit contract.

use crate::{BoundedTimeout, DriverResult, Duration};

/// Writes caller-owned bytes without allocating or waiting indefinitely.
pub trait SerialWrite: BoundedTimeout {
    /// Writes at most `buffer.len()` bytes before the adapter timeout expires.
    fn write(&mut self, buffer: &[u8], timeout: Duration) -> DriverResult<usize>;

    /// Drains accepted bytes before the adapter timeout expires.
    fn flush(&mut self, timeout: Duration) -> DriverResult<()>;
}
