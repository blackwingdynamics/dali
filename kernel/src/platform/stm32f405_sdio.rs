//! STM32 hardware SDIO transport for the F405 board backend.

use super::stm32f405_board::SdioPins;
use core::cell::RefCell;

use super::stm32f405_sdio_raw::RawSdioReader;
use crate::drivers::{Block, BlockAddress, SdioTransport, StorageError};
use stm32f4xx_hal::{
    pac,
    rcc::Clocks,
    sdio::{ClockFreq, SdCard, Sdio},
};

#[repr(C, align(4))]
struct AlignedDmaBlock(Block);

// The section name is part of the F405 linker memory contract: it remains in
// DMA-visible SRAM when ordinary kernel data and stack move to CCM.
#[used]
#[unsafe(link_section = ".dma_buffer")]
static mut DMA_BLOCK: AlignedDmaBlock = AlignedDmaBlock([0; crate::drivers::BLOCK_SIZE]);

/// F405 SDIO transport implementing the generic driver contract.
pub struct Stm32f405SdioTransport {
    device: RefCell<Sdio<SdCard>>,
    raw: RefCell<RawSdioReader>,
}

impl Stm32f405SdioTransport {
    /// Creates an uninitialized F405 SDIO transport.
    pub fn new(peripheral: pac::SDIO, pins: SdioPins, clocks: &Clocks) -> Self {
        Self {
            device: RefCell::new(Sdio::new(peripheral, pins, clocks)),
            raw: RefCell::new(RawSdioReader::new()),
        }
    }
}

impl SdioTransport for Stm32f405SdioTransport {
    /// Initializes the card and switches the bus to the normal transfer clock.
    fn initialize(&mut self) -> Result<u32, StorageError> {
        let mut device = self.device.borrow_mut();
        device.init(ClockFreq::F24Mhz).map_err(map_sdio_error)?;
        let block_count = device.card().map_err(map_sdio_error)?.block_count();
        let card = device.card().map_err(map_sdio_error)?;
        self.raw.borrow_mut().configure(card.capacity);
        Ok(block_count)
    }

    fn read_block(
        &mut self,
        address: BlockAddress,
        buffer: &mut Block,
    ) -> Result<(), StorageError> {
        cortex_m::interrupt::free(|_| unsafe {
            // SAFETY: SDIO is single-owner during bootstrap and the critical
            // section excludes the USB interrupt from this shared DMA buffer.
            let dma_buffer = &mut *core::ptr::addr_of_mut!(DMA_BLOCK.0);
            self.raw.borrow_mut().read_block(address, dma_buffer)?;
            buffer.copy_from_slice(&*core::ptr::addr_of!(DMA_BLOCK.0));
            Ok(())
        })
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
