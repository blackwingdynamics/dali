//! GPIO level and mode contracts.

use crate::DriverResult;

/// Logical GPIO configuration independent of a target register map.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GpioMode {
    /// Digital input mode.
    Input,
    /// Digital push-pull output mode.
    Output,
    /// Peripheral-controlled alternate-function mode.
    Alternate,
}

/// Read-only digital input capability.
pub trait InputPin {
    /// Reads the current logical level.
    fn is_high(&self) -> DriverResult<bool>;
}

/// Digital output capability.
pub trait OutputPin {
    /// Drives the output high.
    fn set_high(&mut self) -> DriverResult<()>;

    /// Drives the output low.
    fn set_low(&mut self) -> DriverResult<()>;

    /// Inverts the current output level.
    fn toggle(&mut self) -> DriverResult<()>;
}

/// Runtime pin-mode capability owned by a platform adapter.
pub trait PinMode {
    /// Changes the pin to a supported logical mode.
    fn set_mode(&mut self, mode: GpioMode) -> DriverResult<()>;
}
