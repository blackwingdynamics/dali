//! STM32 hardware SDIO transport for the F405 board backend.

use super::board::SdioPins;
use core::cell::RefCell;

use super::sdio_raw::RawSdioReader;
use crate::drivers::{Block, BlockAddress, SdioTransport, StorageError};
use stm32f4xx_hal::{
    pac,
    rcc::Clocks,
    sdio::{SdCard, Sdio},
};

/// F405 SDIO transport implementing the generic driver contract.
pub struct Stm32f405SdioTransport {
    _device: RefCell<Sdio<SdCard>>,
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
        self.raw.borrow_mut().initialize()
    }

    fn read_block(
        &mut self,
        address: BlockAddress,
        buffer: &mut Block,
    ) -> Result<(), StorageError> {
        cortex_m::interrupt::free(|_| self.raw.borrow_mut().read_block(address, buffer))
    }

    #[cfg(feature = "storage-write")]
    /// Writes through the HAL CPU/FIFO path without programming DMA2.
    ///
    /// This is the supported F405 write path. The HAL implementation is
    /// bounded by the peripheral's transfer and status handling.
    fn write_block(&mut self, address: BlockAddress, block: &Block) -> Result<(), StorageError> {
        cortex_m::interrupt::free(|_| self.raw.borrow_mut().write_block(address, block))
    }

    #[cfg(feature = "storage-write")]
    fn flush(&mut self) -> Result<(), StorageError> {
        cortex_m::interrupt::free(|_| self.raw.borrow_mut().flush())
    }
}
