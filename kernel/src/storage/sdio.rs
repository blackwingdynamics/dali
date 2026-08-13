//! STM32 hardware SDIO transport for the F405 board backend.

use crate::board::SdioPins;
use crate::storage::{Block, BlockAddress, BlockReader, StorageError};
use stm32f4xx_hal::{
    pac,
    rcc::Clocks,
    sdio::{ClockFreq, SdCard, Sdio},
};

/// A block reader backed by the STM32 SDIO peripheral.
pub struct SdioBlockReader {
    device: Sdio<SdCard>,
}

impl SdioBlockReader {
    /// Creates an uninitialized SDIO block reader.
    pub fn new(peripheral: pac::SDIO, pins: SdioPins, clocks: &Clocks) -> Self {
        Self {
            device: Sdio::new(peripheral, pins, clocks),
        }
    }

    /// Initializes the card and switches the bus to the normal transfer clock.
    pub fn initialize(&mut self) -> Result<(), StorageError> {
        self.device.init(ClockFreq::F24Mhz).map_err(map_sdio_error)
    }
}

impl BlockReader for SdioBlockReader {
    fn read_block(
        &mut self,
        address: BlockAddress,
        buffer: &mut Block,
    ) -> Result<(), StorageError> {
        self.device
            .read_block(address.value(), buffer)
            .map_err(map_sdio_error)
    }
}

fn map_sdio_error(error: stm32f4xx_hal::sdio::Error) -> StorageError {
    match error {
        stm32f4xx_hal::sdio::Error::NoCard => StorageError::NotReady,
        stm32f4xx_hal::sdio::Error::Timeout | stm32f4xx_hal::sdio::Error::SoftwareTimeout => {
            StorageError::Timeout
        }
        stm32f4xx_hal::sdio::Error::Crc | stm32f4xx_hal::sdio::Error::DataCrcFail => {
            StorageError::DataCorruption
        }
        stm32f4xx_hal::sdio::Error::UnsupportedCardVersion
        | stm32f4xx_hal::sdio::Error::UnsupportedCardType
        | stm32f4xx_hal::sdio::Error::UnsupportedVoltage => StorageError::Unsupported,
        stm32f4xx_hal::sdio::Error::RxOverFlow | stm32f4xx_hal::sdio::Error::TxUnderErr => {
            StorageError::Transport
        }
    }
}
