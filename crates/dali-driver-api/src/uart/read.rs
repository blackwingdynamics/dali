//! Serial receive contract.

use crate::{BoundedTimeout, DriverResult, Duration};

/// Reads caller-owned bytes without allocating or waiting indefinitely.
pub trait SerialRead: BoundedTimeout {
    /// Reads at most `buffer.len()` bytes before the adapter timeout expires.
    fn read(&mut self, buffer: &mut [u8], timeout: Duration) -> DriverResult<usize>;
}
