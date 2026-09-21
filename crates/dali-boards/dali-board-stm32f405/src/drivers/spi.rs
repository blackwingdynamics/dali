//! Bounded SPI adapter for the F405 platform boundary.

use super::timeout::F405TimeoutConfig;
use dali_driver_api::{
    BoundedTimeout, DriverError, DriverResult, Duration, SpiBusOwnership, SpiDeviceId,
    SpiDeviceSelect, SpiTransfer,
};
use stm32f4xx_hal::{
    hal_02::spi::FullDuplex,
    nb::Error as NbError,
    spi::{Instance, Spi},
};

/// SPI master adapter that exposes only the hardware-neutral transfer contract.
pub(crate) struct F405Spi<SPI: Instance> {
    spi: Spi<SPI, false, u8>,
    timeout: F405TimeoutConfig,
    owned: bool,
    selected: Option<SpiDeviceId>,
}

impl<SPI: Instance> F405Spi<SPI> {
    /// Wraps an initialized HAL SPI master and its timeout policy.
    pub(crate) const fn new(spi: Spi<SPI, false, u8>, timeout: F405TimeoutConfig) -> Self {
        Self {
            spi,
            timeout,
            owned: false,
            selected: None,
        }
    }

    /// Rejects transfers that do not hold both bus and device ownership.
    fn require_selection(&self) -> DriverResult<()> {
        if self.owned && self.selected.is_some() {
            Ok(())
        } else {
            Err(DriverError::InvalidState)
        }
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

    /// Disables the peripheral to create a bounded, recoverable stalled probe.
    pub(crate) fn disable_for_probe(&mut self) {
        self.spi.enable(false);
    }

    /// Re-enables the peripheral after the intentional stalled probe.
    pub(crate) fn enable_for_probe(&mut self) {
        self.spi.enable(true);
    }
}

impl<SPI: Instance> BoundedTimeout for F405Spi<SPI> {
    fn max_timeout(&self) -> Duration {
        self.timeout.max_timeout()
    }
}

impl<SPI: Instance> SpiBusOwnership for F405Spi<SPI> {
    fn acquire(&mut self) -> DriverResult<()> {
        if self.owned {
            Err(DriverError::ResourceBusy)
        } else {
            self.owned = true;
            Ok(())
        }
    }

    fn release(&mut self) -> DriverResult<()> {
        if !self.owned || self.selected.is_some() {
            Err(DriverError::InvalidState)
        } else {
            self.owned = false;
            Ok(())
        }
    }

    fn is_owned(&self) -> bool {
        self.owned
    }
}

impl<SPI: Instance> SpiDeviceSelect for F405Spi<SPI> {
    fn select(&mut self, device: SpiDeviceId) -> DriverResult<()> {
        if !self.owned {
            return Err(DriverError::InvalidState);
        }
        if self.selected.is_some() {
            return Err(DriverError::ResourceBusy);
        }
        self.selected = Some(device);
        Ok(())
    }

    fn deselect(&mut self) -> DriverResult<()> {
        if !self.owned || self.selected.take().is_none() {
            Err(DriverError::InvalidState)
        } else {
            Ok(())
        }
    }

    fn selected(&self) -> Option<SpiDeviceId> {
        self.selected
    }
}

impl<SPI: Instance> SpiTransfer for F405Spi<SPI> {
    fn transfer(&mut self, buffer: &mut [u8], timeout: Duration) -> DriverResult<()> {
        self.require_selection()?;
        let mut remaining = self.timeout.poll_budget(timeout)?;
        for byte in buffer.iter_mut() {
            *byte = self.exchange_byte(*byte, &mut remaining)?;
        }
        Ok(())
    }
}
