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
pub use spi::SpiTransfer;
pub use timer::{BoundedTimeout, CountDown, Duration, TimerDriver};
pub use uart::{SerialRead, SerialWrite};
