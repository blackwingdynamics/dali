//! Bounded SPI adapter for the F405 platform boundary.

use super::timeout::F405TimeoutConfig;
use dali_driver_api::{BoundedTimeout, DriverError, DriverResult, Duration, SpiTransfer};
use stm32f4xx_hal::{
    hal_02::spi::FullDuplex,
    nb::Error as NbError,
    spi::{Instance, Spi},
};

/// SPI master adapter that exposes only the hardware-neutral transfer contract.
pub(crate) struct F405Spi<SPI: Instance> {
    spi: Spi<SPI, false, u8>,
    timeout: F405TimeoutConfig,
}

impl<SPI: Instance> F405Spi<SPI> {
    /// Wraps an initialized HAL SPI master and its timeout policy.
    pub(crate) const fn new(spi: Spi<SPI, false, u8>, timeout: F405TimeoutConfig) -> Self {
        Self { spi, timeout }
    }

    /// Maps a HAL SPI error into the hardware-neutral driver vocabulary.
    fn map_error(error: stm32f4xx_hal::spi::Error) -> DriverError {
        match error {
            stm32f4xx_hal::spi::Error::Overrun => DriverError::Overrun,
            stm32f4xx_hal::spi::Error::ModeFault | stm32f4xx_hal::spi::Error::Crc => {
                DriverError::HardwareFault
            }
            _ => DriverError::HardwareFault,
        }
    }

    /// Polls one full-duplex byte exchange with a finite retry budget.
    fn exchange_byte(&mut self, byte: u8, remaining: &mut u32) -> DriverResult<u8> {
        while *remaining != 0 {
            match FullDuplex::send(&mut self.spi, byte) {
                Ok(()) => break,
                Err(NbError::WouldBlock) => {
                    *remaining -= 1;
                    continue;
                }
                Err(NbError::Other(error)) => return Err(Self::map_error(error)),
            }
        }
        if *remaining == 0 {
            return Err(DriverError::Timeout);
        }

        while *remaining != 0 {
            match FullDuplex::read(&mut self.spi) {
                Ok(response) => return Ok(response),
                Err(NbError::WouldBlock) => *remaining -= 1,
                Err(NbError::Other(error)) => return Err(Self::map_error(error)),
            }
        }
        Err(DriverError::Timeout)
    }
}

impl<SPI: Instance> BoundedTimeout for F405Spi<SPI> {
    fn max_timeout(&self) -> Duration {
        self.timeout.max_timeout()
    }
}

impl<SPI: Instance> SpiTransfer for F405Spi<SPI> {
    fn transfer(&mut self, buffer: &mut [u8], timeout: Duration) -> DriverResult<()> {
        let mut remaining = self.timeout.poll_budget(timeout)?;
        for byte in buffer.iter_mut() {
            *byte = self.exchange_byte(*byte, &mut remaining)?;
        }
        Ok(())
    }
}
