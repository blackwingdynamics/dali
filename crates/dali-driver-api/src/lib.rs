#![no_std]
#![warn(missing_docs)]

//! Hardware-neutral, allocation-free driver contracts.

pub mod display;
pub mod error;
pub mod gpio;
pub mod i2c;
pub mod spi;
pub mod timer;
pub mod uart;

pub use display::{
    DiagnosticsConsole, DisplayDimensions, DisplayDriver, TextModeProperties, TextPosition,
};
pub use error::{DriverError, DriverResult};
pub use gpio::{
    GpioMode, InputPin, InterruptCallback, InterruptPin, InterruptTrigger, OutputPin, PinMode,
};
pub use i2c::{I2cAddress, I2cDriver};
pub use spi::{SpiBusOwnership, SpiDeviceId, SpiDeviceSelect, SpiTransfer};
pub use timer::{BoundedTimeout, CountDown, Duration, TimerDriver};
pub use uart::{
    BaudRate, DataBits, Parity, SerialConfig, SerialConfigure, SerialOwnership, SerialRead,
    SerialWrite, StopBits,
};
