//! F405 adapters for the hardware-neutral driver contracts.

mod gpio;
pub(crate) mod interrupt;
#[cfg(feature = "f405-serial-spi")]
mod probe;
#[cfg(feature = "f405-serial-spi")]
mod spi;
#[cfg(feature = "f405-serial-spi")]
mod timeout;
#[cfg(feature = "abi-context-switch")]
mod timer;
#[cfg(feature = "f405-serial-spi")]
mod uart;

pub(crate) use gpio::F405GpioPin;
pub(crate) use interrupt::F405ExtiPin;
#[cfg(feature = "f405-serial-spi")]
pub(crate) use probe::{F405DriverProbe, F405DriverProbeResult};
#[cfg(feature = "f405-serial-spi")]
pub(crate) use spi::F405Spi;
#[cfg(feature = "f405-serial-spi")]
pub(crate) use timeout::F405TimeoutConfig;
#[cfg(feature = "abi-context-switch")]
pub(crate) use timer::F405TimerDriver;
#[cfg(feature = "f405-serial-spi")]
pub(crate) use uart::F405Uart;
