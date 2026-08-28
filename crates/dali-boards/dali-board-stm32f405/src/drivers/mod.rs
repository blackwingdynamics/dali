//! F405 adapters for the hardware-neutral driver contracts.

mod gpio;
#[cfg(any(feature = "f405-serial-spi", feature = "display-oled"))]
mod i2c;
pub(crate) mod interrupt;
#[cfg(feature = "driver-hardware-test")]
mod probe;
#[cfg(feature = "driver-hardware-test")]
mod spi;
#[cfg(feature = "display-oled")]
pub(crate) mod ssd1306;
#[cfg(any(feature = "f405-serial-spi", feature = "display-oled"))]
mod timeout;
#[cfg(feature = "abi-context-switch")]
mod timer;
#[cfg(feature = "driver-hardware-test")]
mod uart;

pub(crate) use gpio::F405GpioPin;
#[cfg(any(feature = "f405-serial-spi", feature = "display-oled"))]
pub(crate) use i2c::F405I2c;
pub(crate) use interrupt::F405ExtiPin;
#[cfg(feature = "driver-hardware-test")]
pub(crate) use probe::{F405DriverProbe, F405DriverProbeResult};
#[cfg(feature = "driver-hardware-test")]
pub(crate) use spi::F405Spi;
#[cfg(any(feature = "f405-serial-spi", feature = "display-oled"))]
pub(crate) use timeout::F405TimeoutConfig;
#[cfg(feature = "abi-context-switch")]
pub(crate) use timer::F405TimerDriver;
#[cfg(feature = "driver-hardware-test")]
pub(crate) use uart::F405Uart;
