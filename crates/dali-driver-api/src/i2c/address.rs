//! Hardware-neutral I2C address type.

/// A logical I2C target address.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct I2cAddress(u8);

impl I2cAddress {
    /// Creates an address value without exposing a platform register map.
    pub const fn new(value: u8) -> Self {
        Self(value)
    }

    /// Returns the address value supplied by the bus configuration.
    pub const fn value(self) -> u8 {
        self.0
    }
}
