//! F405 adapters for the hardware-neutral driver contracts.

mod gpio;
#[cfg(feature = "abi-context-switch")]
mod timer;

pub(crate) use gpio::F405GpioPin;
#[cfg(feature = "abi-context-switch")]
pub(crate) use timer::F405TimerDriver;
