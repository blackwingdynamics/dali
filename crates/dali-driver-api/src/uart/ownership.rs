//! Exclusive UART ownership contract.

use crate::DriverResult;

/// Exclusive ownership lifecycle for one UART resource.
pub trait SerialOwnership {
    /// Claims the UART for the current caller.
    ///
    /// # Errors
    ///
    /// Returns [`crate::DriverError::ResourceBusy`] when another caller owns it.
    fn acquire(&mut self) -> DriverResult<()>;

    /// Releases a previously claimed UART.
    ///
    /// # Errors
    ///
    /// Returns [`crate::DriverError::InvalidState`] when the UART is not owned.
    fn release(&mut self) -> DriverResult<()>;

    /// Reports whether this adapter currently has an owner.
    fn is_owned(&self) -> bool;
}
