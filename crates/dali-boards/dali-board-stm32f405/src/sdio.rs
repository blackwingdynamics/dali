//! STM32 hardware SDIO transport for the F405 board backend.

use super::board::SdioPins;
use core::cell::RefCell;

use super::sdio_raw::RawSdioReader;
use dali_kernel_api::storage::{
    Block, BlockAddress, BlockReader, SdioTransport, StorageError, StorageLifecycleControl,
};
use stm32f4xx_hal::{
    pac,
    rcc::Clocks,
    sdio::{SdCard, Sdio},
};

/// F405 SDIO transport implementing the generic driver contract.
pub struct Stm32f405SdioTransport {
    /// Stores the device associated with this bounded state.
    _device: RefCell<Sdio<SdCard>>,
    /// Stores the raw associated with this bounded state.
    raw: RefCell<RawSdioReader>,
}

impl Stm32f405SdioTransport {
    /// Creates an uninitialized F405 SDIO transport.
    pub fn new(peripheral: pac::SDIO, pins: SdioPins, clocks: &Clocks) -> Self {
        Self {
            _device: RefCell::new(Sdio::new(peripheral, pins, clocks)),
            raw: RefCell::new(RawSdioReader::new()),
        }
    }
}

impl SdioTransport for Stm32f405SdioTransport {
    /// Initializes the card and switches the bus to the normal transfer clock.
    fn initialize(&mut self) -> Result<u32, StorageError> {
        self.raw.borrow_mut().initialize().map_err(classify_timeout)
    }

    fn read_block(
        &mut self,
        address: BlockAddress,
        buffer: &mut Block,
    ) -> Result<(), StorageError> {
        cortex_m::interrupt::free(|_| self.raw.borrow_mut().read_block(address, buffer))
            .map_err(classify_timeout)
    }

    /// Writes through the HAL CPU/FIFO path without programming DMA2.
    ///
    /// This is the supported F405 write path. The HAL implementation is
    /// bounded by the peripheral's transfer and status handling.
    #[cfg(feature = "storage-write")]
    fn write_block(&mut self, address: BlockAddress, block: &Block) -> Result<(), StorageError> {
        cortex_m::interrupt::free(|_| self.raw.borrow_mut().write_block(address, block))
            .map_err(classify_timeout)
    }

    #[cfg(not(feature = "storage-write"))]
    fn write_block(&mut self, _address: BlockAddress, _block: &Block) -> Result<(), StorageError> {
        Err(StorageError::Unsupported)
    }

    #[cfg(feature = "storage-write")]
    fn flush(&mut self) -> Result<(), StorageError> {
        cortex_m::interrupt::free(|_| self.raw.borrow_mut().flush()).map_err(classify_timeout)
    }

    #[cfg(not(feature = "storage-write"))]
    fn flush(&mut self) -> Result<(), StorageError> {
        Err(StorageError::Unsupported)
    }
}

/// Lifecycle-aware block reader composed from the F405 SDIO transport.
pub struct SdioBlockReader {
    /// Transport owned by this reader.
    transport: Stm32f405SdioTransport,
    /// Capacity reported by the initialized card.
    block_count: Option<u32>,
}

impl SdioBlockReader {
    /// Creates an uninitialized reader around the board transport.
    pub const fn new(transport: Stm32f405SdioTransport) -> Self {
        Self {
            transport,
            block_count: None,
        }
    }

    fn initialize_inner(&mut self) -> Result<(), StorageError> {
        self.block_count = Some(self.transport.initialize()?);
        Ok(())
    }
}

impl StorageLifecycleControl for SdioBlockReader {
    fn initialize(&mut self) -> Result<(), StorageError> {
        self.initialize_inner()
    }

    fn reinitialize(&mut self) -> Result<(), StorageError> {
        self.initialize_inner()
    }
}

impl BlockReader for SdioBlockReader {
    fn read_block(
        &mut self,
        address: BlockAddress,
        buffer: &mut Block,
    ) -> Result<(), StorageError> {
        if self.block_count.is_none() {
            return Err(StorageError::NotReady);
        }
        self.transport.read_block(address, buffer)
    }

    fn block_count(&self) -> Result<u32, StorageError> {
        self.block_count.ok_or(StorageError::NotReady)
    }
}

impl dali_kernel_api::storage::BlockWriter for SdioBlockReader {
    #[cfg(not(feature = "storage-write"))]
    fn write_block(&mut self, _address: BlockAddress, _block: &Block) -> Result<(), StorageError> {
        Err(StorageError::Unsupported)
    }

    #[cfg(feature = "storage-write")]
    fn write_block(&mut self, address: BlockAddress, block: &Block) -> Result<(), StorageError> {
        self.transport.write_block(address, block)
    }
}

impl dali_kernel_api::storage::BlockTransportFlush for SdioBlockReader {
    type Error = StorageError;

    #[cfg(not(feature = "storage-write"))]
    fn flush(&mut self) -> Result<(), Self::Error> {
        Err(StorageError::Unsupported)
    }

    #[cfg(feature = "storage-write")]
    fn flush(&mut self) -> Result<(), Self::Error> {
        self.transport.flush()
    }
}

/// The F405 board has no card-detect GPIO; a bounded SDIO timeout is its
/// target-observable indication that the medium stopped responding.
fn classify_timeout(error: StorageError) -> StorageError {
    if matches!(error, StorageError::Timeout) {
        StorageError::CardRemoved
    } else {
        error
    }
}
