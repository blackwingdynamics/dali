//! Hardware-neutral bounded I2C driver contract.

use crate::{BoundedTimeout, DriverResult, Duration, I2cAddress};

/// Owns one I2C bus and performs bounded transactions against logical targets.
pub trait I2cDriver: BoundedTimeout {
    /// Claims exclusive ownership of the bus.
    ///
    /// # Errors
    ///
    /// Returns [`crate::DriverError::ResourceBusy`] when another caller owns
    /// the bus.
    fn acquire(&mut self) -> DriverResult<()>;

    /// Releases exclusive ownership of the bus after a completed transaction.
    ///
    /// # Errors
    ///
    /// Returns [`crate::DriverError::InvalidState`] when the bus is not owned.
    fn release(&mut self) -> DriverResult<()>;

    /// Reports whether this driver currently owns the bus.
    fn is_owned(&self) -> bool;

    /// Writes caller-owned bytes to one logical target.
    ///
    /// # Errors
    ///
    /// Returns a typed driver error when the target rejects the transfer, the
    /// bus loses arbitration, the bus reports a fault, or the timeout expires.
    fn write(&mut self, address: I2cAddress, bytes: &[u8], timeout: Duration) -> DriverResult<()>;

    /// Reads bytes into a caller-owned buffer from one logical target.
    ///
    /// # Errors
    ///
    /// Returns a typed driver error when the target rejects the transfer, the
    /// bus loses arbitration, the bus reports a fault, or the timeout expires.
    fn read(
        &mut self,
        address: I2cAddress,
        buffer: &mut [u8],
        timeout: Duration,
    ) -> DriverResult<()>;

    /// Writes and then reads using one transaction with a repeated start.
    ///
    /// The adapter must keep bus ownership across both phases and must not
    /// insert a stop condition between `write_bytes` and `read_buffer`.
    ///
    /// # Errors
    ///
    /// Returns a typed driver error when either phase fails, the bus loses
    /// arbitration, the bus reports a fault, or the timeout expires.
    fn write_read(
        &mut self,
        address: I2cAddress,
        write_bytes: &[u8],
        read_buffer: &mut [u8],
        timeout: Duration,
    ) -> DriverResult<()>;
}
