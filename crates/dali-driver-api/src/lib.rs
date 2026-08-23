#![no_std]

//! Hardware-neutral, allocation-free driver contracts.

pub mod error;
pub mod gpio;
pub mod timer;

pub use error::{DriverError, DriverResult};
pub use gpio::{GpioMode, InputPin, InterruptPin, InterruptTrigger, OutputPin, PinMode};
pub use timer::{BoundedTimeout, CountDown, Duration, TimerDriver};
