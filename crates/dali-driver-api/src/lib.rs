#![no_std]
#![warn(missing_docs)]

//! Hardware-neutral, allocation-free driver contracts.

pub mod error;
pub mod gpio;
pub mod spi;
pub mod timer;
pub mod uart;

pub use error::{DriverError, DriverResult};
pub use gpio::{
    GpioMode, InputPin, InterruptCallback, InterruptPin, InterruptTrigger, OutputPin, PinMode,
};
pub use spi::{SpiBusOwnership, SpiDeviceId, SpiDeviceSelect, SpiTransfer};
pub use timer::{BoundedTimeout, CountDown, Duration, TimerDriver};
pub use uart::{
    BaudRate, DataBits, Parity, SerialConfig, SerialConfigure, SerialOwnership, SerialRead,
    SerialWrite, StopBits,
};
