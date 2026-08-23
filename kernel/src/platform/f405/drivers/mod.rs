//! F405 adapters for the hardware-neutral driver contracts.

mod gpio;
pub(crate) mod interrupt;
#[cfg(feature = "abi-context-switch")]
mod timer;

pub(crate) use gpio::F405GpioPin;
pub(crate) use interrupt::F405ExtiPin;
#[cfg(feature = "abi-context-switch")]
pub(crate) use timer::F405TimerDriver;
