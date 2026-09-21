//! Bounded UART adapter for the F405 platform boundary.

use super::timeout::F405TimeoutConfig;
use dali_driver_api::{
    BoundedTimeout, DriverError, DriverResult, Duration, SerialConfig, SerialConfigure,
    SerialOwnership, SerialRead, SerialWrite,
};
use stm32f4xx_hal::{
    hal_02::serial::{Read as SerialReadNb, Write as SerialWriteNb},
    nb::Error as NbError,
    serial::Serial1,
};

/// UART adapter that exposes only the hardware-neutral serial contracts.
pub(crate) struct F405Uart {
    serial: Serial1,
    timeout: F405TimeoutConfig,
    config: SerialConfig,
    owned: bool,
}

impl F405Uart {
    /// Wraps an initialized HAL UART and its target-specific timeout policy.
    pub(crate) const fn new(
        serial: Serial1,
        timeout: F405TimeoutConfig,
        config: SerialConfig,
    ) -> Self {
        Self {
            serial,
            timeout,
            config,
            owned: false,
        }
    }

    /// Rejects I/O from callers that have not claimed the UART resource.
    fn require_ownership(&self) -> DriverResult<()> {
        if self.owned {
            Ok(())
        } else {
            Err(DriverError::InvalidState)
        }
    }

    /// Maps a HAL UART error into the hardware-neutral driver vocabulary.
    fn map_error(error: stm32f4xx_hal::serial::Error) -> DriverError {
        match error {
            stm32f4xx_hal::serial::Error::Overrun => DriverError::Overrun,
            stm32f4xx_hal::serial::Error::FrameFormat => DriverError::Framing,
            stm32f4xx_hal::serial::Error::Parity => DriverError::Parity,
            stm32f4xx_hal::serial::Error::Noise | stm32f4xx_hal::serial::Error::Other => {
                DriverError::HardwareFault
            }
            _ => DriverError::HardwareFault,
        }
    }

    /// Polls one byte with a strictly finite retry budget.
    fn read_byte(&mut self, remaining: &mut u32) -> DriverResult<u8> {
        while *remaining != 0 {
            match SerialReadNb::read(&mut self.serial) {
                Ok(byte) => return Ok(byte),
                Err(NbError::WouldBlock) => *remaining -= 1,
                Err(NbError::Other(error)) => return Err(Self::map_error(error)),
            }
        }
        Err(DriverError::Timeout)
    }

    /// Polls one byte transmission with a strictly finite retry budget.
    fn write_byte(&mut self, byte: u8, remaining: &mut u32) -> DriverResult<()> {
        while *remaining != 0 {
            match SerialWriteNb::write(&mut self.serial, byte) {
                Ok(()) => return Ok(()),
                Err(NbError::WouldBlock) => *remaining -= 1,
                Err(NbError::Other(error)) => return Err(Self::map_error(error)),
            }
        }
        Err(DriverError::Timeout)
    }
}

impl BoundedTimeout for F405Uart {
    fn max_timeout(&self) -> Duration {
        self.timeout.max_timeout()
    }
}

impl SerialOwnership for F405Uart {
    fn acquire(&mut self) -> DriverResult<()> {
        if self.owned {
            Err(DriverError::ResourceBusy)
        } else {
            self.owned = true;
            Ok(())
        }
    }

    fn release(&mut self) -> DriverResult<()> {
        if !self.owned {
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

impl SerialConfigure for F405Uart {
    fn configure(&mut self, config: SerialConfig) -> DriverResult<()> {
        self.require_ownership()?;
        if config == self.config {
            Ok(())
        } else {
            // stm32f4xx-hal configures Serial1 while constructing the HAL
            // resource; changing registers after construction is not exposed
            // by this backend, so incompatible requests are rejected.
            Err(DriverError::Unsupported)
        }
    }
}

impl SerialRead for F405Uart {
    fn read(&mut self, buffer: &mut [u8], timeout: Duration) -> DriverResult<usize> {
        self.require_ownership()?;
        let mut remaining = self.timeout.poll_budget(timeout)?;
        for byte in buffer.iter_mut() {
            *byte = self.read_byte(&mut remaining)?;
        }
        Ok(buffer.len())
    }
}

impl SerialWrite for F405Uart {
    fn write(&mut self, buffer: &[u8], timeout: Duration) -> DriverResult<usize> {
        self.require_ownership()?;
        let mut remaining = self.timeout.poll_budget(timeout)?;
        for &byte in buffer {
            self.write_byte(byte, &mut remaining)?;
        }
        Ok(buffer.len())
    }

    fn flush(&mut self, timeout: Duration) -> DriverResult<()> {
        self.require_ownership()?;
        let mut remaining = self.timeout.poll_budget(timeout)?;
        while remaining != 0 {
            match SerialWriteNb::flush(&mut self.serial) {
                Ok(()) => return Ok(()),
                Err(NbError::WouldBlock) => remaining -= 1,
                Err(NbError::Other(error)) => return Err(Self::map_error(error)),
            }
        }
        Err(DriverError::Timeout)
    }
}
