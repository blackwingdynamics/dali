//! SPI transfer contract.

use crate::{BoundedTimeout, DriverResult, Duration};

/// Performs one full-duplex transfer in a caller-owned buffer.
pub trait SpiTransfer: BoundedTimeout {
    /// Exchanges all bytes in `buffer` before the adapter timeout expires.
    fn transfer(&mut self, buffer: &mut [u8], timeout: Duration) -> DriverResult<()>;
}
