//! STM32 hardware SDIO transport for the F405 board backend.

use crate::board::SdioPins;
use core::cell::{Cell, RefCell};

use crate::storage::{Block, BlockAddress, BlockReader, StorageError};
use embedded_sdmmc::{Block as FilesystemBlock, BlockCount, BlockDevice, BlockIdx};
use stm32f4xx_hal::{
    pac,
    rcc::Clocks,
    sdio::{ClockFreq, SdCard, Sdio},
};

/// A block reader backed by the STM32 SDIO peripheral.
pub struct SdioBlockReader {
    device: RefCell<Sdio<SdCard>>,
    block_count: Cell<Option<u32>>,
}

impl SdioBlockReader {
    /// Creates an uninitialized SDIO block reader.
    pub fn new(peripheral: pac::SDIO, pins: SdioPins, clocks: &Clocks) -> Self {
        Self {
            device: RefCell::new(Sdio::new(peripheral, pins, clocks)),
            block_count: Cell::new(None),
        }
    }

    /// Initializes the card and switches the bus to the normal transfer clock.
    pub fn initialize(&mut self) -> Result<(), StorageError> {
        let mut device = self.device.borrow_mut();
        device.init(ClockFreq::F24Mhz).map_err(map_sdio_error)?;
        let block_count = device.card().map_err(map_sdio_error)?.block_count();
        self.block_count.set(Some(block_count));
        Ok(())
    }
}

impl BlockReader for SdioBlockReader {
    fn read_block(
        &mut self,
        address: BlockAddress,
        buffer: &mut Block,
    ) -> Result<(), StorageError> {
        self.device
            .borrow_mut()
            .read_block(address.value(), buffer)
            .map_err(map_sdio_error)
    }
}

impl BlockDevice for SdioBlockReader {
    type Error = StorageError;

    fn read(
        &self,
        blocks: &mut [FilesystemBlock],
        start_block_idx: BlockIdx,
    ) -> Result<(), Self::Error> {
        let mut device = self.device.borrow_mut();
        for (offset, block) in blocks.iter_mut().enumerate() {
            let offset = u32::try_from(offset).map_err(|_| StorageError::InvalidBlockAddress)?;
            let address = start_block_idx
                .0
                .checked_add(offset)
                .ok_or(StorageError::InvalidBlockAddress)?;
            device
                .read_block(address, &mut block.contents)
                .map_err(map_sdio_error)?;
        }
        Ok(())
    }

    fn write(
        &self,
        _blocks: &[FilesystemBlock],
        _start_block_idx: BlockIdx,
    ) -> Result<(), Self::Error> {
        Err(StorageError::Unsupported)
    }

    fn num_blocks(&self) -> Result<BlockCount, Self::Error> {
        self.block_count
            .get()
            .map(BlockCount)
            .ok_or(StorageError::NotReady)
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
