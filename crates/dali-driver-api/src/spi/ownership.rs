//! Exclusive SPI bus and chip-select contracts.

use crate::DriverResult;

/// Opaque device selector supplied by board configuration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SpiDeviceId(u8);

impl SpiDeviceId {
    /// Creates a device selector without exposing a board register map.
    pub const fn new(value: u8) -> Self {
        Self(value)
    }

    /// Returns the configured device selector value.
    pub const fn value(self) -> u8 {
        self.0
    }
}

/// Exclusive ownership lifecycle for one SPI bus.
pub trait SpiBusOwnership {
    /// Claims the bus for one bounded transaction sequence.
    ///
    /// # Errors
    ///
    /// Returns [`crate::DriverError::ResourceBusy`] when another caller owns it.
    fn acquire(&mut self) -> DriverResult<()>;

    /// Releases a previously claimed SPI bus.
    ///
    /// # Errors
    ///
    /// Returns [`crate::DriverError::InvalidState`] when the bus is not owned.
    fn release(&mut self) -> DriverResult<()>;

    /// Reports whether this adapter currently owns the bus.
    fn is_owned(&self) -> bool;
}

/// Device-selection lifecycle for an owned SPI bus.
pub trait SpiDeviceSelect {
    /// Asserts the selected device for the next bounded transfer sequence.
    ///
    /// # Errors
    ///
    /// Returns [`crate::DriverError::InvalidState`] when the bus is not owned
    /// and [`crate::DriverError::ResourceBusy`] when another device is already
    /// selected.
    fn select(&mut self, device: SpiDeviceId) -> DriverResult<()>;

    /// Deasserts the currently selected device.
    ///
    /// # Errors
    ///
    /// Returns [`crate::DriverError::InvalidState`] when no device is selected.
    fn deselect(&mut self) -> DriverResult<()>;

    /// Returns the currently selected device, if any.
    fn selected(&self) -> Option<SpiDeviceId>;
}
