//! Bounded I2C1 adapter for the F405 platform boundary.

use super::timeout::F405TimeoutConfig;
use dali_driver_api::{BoundedTimeout, DriverError, DriverResult, Duration, I2cAddress, I2cDriver};
use stm32f4xx_hal::{
    gpio::{Alternate, gpiob},
    pac,
    rcc::Clocks,
};

mod operations;

/// Unit conversion used by the I2C peripheral clock register.
const HZ_PER_MHZ: u32 = 1_000_000;
/// Standard-mode CCR divider for the F405 I2C peripheral.
const STANDARD_CCR_DIVISOR: u32 = 2;
/// Minimum standard-mode CCR value accepted by the peripheral.
const MIN_STANDARD_CCR: u32 = 4;
/// Rise-time register margin required by standard-mode I2C.
const TRISE_MARGIN: u8 = 1;
/// Maximum I2C peripheral clock frequency accepted by the F405 timing layout.
const MAX_I2C_CLOCK_MHZ: u32 = 50;
/// Minimum I2C peripheral clock frequency accepted by the F405 timing layout.
const MIN_I2C_CLOCK_MHZ: u32 = 2;

/// F405 I2C1 adapter with exclusive bus ownership and bounded register polls.
pub(crate) struct F405I2c {
    peripheral: pac::I2C1,
    _pins: (gpiob::PB6<Alternate<4>>, gpiob::PB7<Alternate<4>>),
    timeout: F405TimeoutConfig,
    pclk1_hz: u32,
    bus_frequency_hz: u32,
    owned: bool,
    recovery_observed: bool,
}

impl F405I2c {
    /// Creates and initializes I2C1 on the board-owned PB6/PB7 pins.
    pub(crate) fn new(
        peripheral: pac::I2C1,
        pins: (gpiob::PB6<Alternate<4>>, gpiob::PB7<Alternate<4>>),
        clocks: &Clocks,
        timeout: F405TimeoutConfig,
        bus_frequency_hz: u32,
    ) -> Option<Self> {
        let pclk1_hz = clocks.pclk1().raw();
        let adapter = Self {
            peripheral,
            _pins: pins,
            timeout,
            pclk1_hz,
            bus_frequency_hz,
            owned: false,
            recovery_observed: false,
        };
        adapter.configure().ok()?;
        Some(adapter)
    }

    fn configure(&self) -> DriverResult<()> {
        let clock_mhz = self.pclk1_hz / HZ_PER_MHZ;
        if !(MIN_I2C_CLOCK_MHZ..=MAX_I2C_CLOCK_MHZ).contains(&clock_mhz) {
            return Err(DriverError::HardwareFault);
        }
        self.peripheral
            .cr1
            .modify(|_, writer| writer.pe().clear_bit());
        self.peripheral
            .cr2
            // SAFETY: clock_mhz is bounded by the F405 I2C timing limits above.
            .write(|writer| unsafe { writer.freq().bits(clock_mhz as u8) });
        self.peripheral
            .trise
            .write(|writer| writer.trise().bits(clock_mhz as u8 + TRISE_MARGIN));
        let divider = self
            .bus_frequency_hz
            .checked_mul(STANDARD_CCR_DIVISOR)
            .ok_or(DriverError::HardwareFault)?;
        let ccr = (self.pclk1_hz / divider).max(MIN_STANDARD_CCR);
        let ccr = u16::try_from(ccr).map_err(|_| DriverError::HardwareFault)?;
        self.peripheral
            .ccr
            // SAFETY: ccr is range-checked before writing the standard-mode field.
            .write(|writer| unsafe { writer.f_s().clear_bit().duty().clear_bit().ccr().bits(ccr) });
        self.peripheral
            .cr1
            .modify(|_, writer| writer.ack().set_bit().pe().set_bit());
        Ok(())
    }

    fn recover(&mut self) -> DriverResult<()> {
        self.peripheral
            .cr1
            .modify(|_, writer| writer.pe().clear_bit().swrst().set_bit());
        self.peripheral
            .cr1
            .modify(|_, writer| writer.swrst().clear_bit());
        let result = self.configure();
        if result.is_ok() {
            self.recovery_observed = true;
        }
        result
    }

    /// Returns whether a bounded operation completed a peripheral recovery.
    #[cfg(feature = "driver-hardware-test")]
    pub(crate) fn recovery_observed(&mut self) -> bool {
        let observed = self.recovery_observed;
        self.recovery_observed = false;
        observed
    }
}

impl BoundedTimeout for F405I2c {
    fn max_timeout(&self) -> Duration {
        self.timeout.max_timeout()
    }
}

impl I2cDriver for F405I2c {
    fn acquire(&mut self) -> DriverResult<()> {
        if self.owned {
            Err(DriverError::ResourceBusy)
        } else {
            self.owned = true;
            Ok(())
        }
    }

    fn release(&mut self) -> DriverResult<()> {
        if self.owned {
            self.owned = false;
            Ok(())
        } else {
            Err(DriverError::InvalidState)
        }
    }

    fn is_owned(&self) -> bool {
        self.owned
    }

    fn write(&mut self, address: I2cAddress, bytes: &[u8], timeout: Duration) -> DriverResult<()> {
        if !self.owned {
            return Err(DriverError::InvalidState);
        }
        self.execute_write(address, bytes, timeout)
    }

    fn read(
        &mut self,
        address: I2cAddress,
        buffer: &mut [u8],
        timeout: Duration,
    ) -> DriverResult<()> {
        if !self.owned {
            return Err(DriverError::InvalidState);
        }
        self.execute_read(address, buffer, timeout)
    }

    fn write_read(
        &mut self,
        address: I2cAddress,
        write_bytes: &[u8],
        read_buffer: &mut [u8],
        timeout: Duration,
    ) -> DriverResult<()> {
        if !self.owned {
            return Err(DriverError::InvalidState);
        }
        self.execute_write_read(address, write_bytes, read_buffer, timeout)
    }
}
