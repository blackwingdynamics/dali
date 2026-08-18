//! Bounded SDIO block reads for the STM32F4 data path.

use crate::drivers::{Block, BlockAddress, StorageError};
use stm32f4xx_hal::pac;
use stm32f4xx_hal::sdio::CardCapacity;

mod status;
use status::{clear_interrupts, status_error};

const BLOCK_BYTES: usize = 512;
const BLOCK_SIZE_EXPONENT: u8 = 9;
const DATA_TIMEOUT_CYCLES: u32 = u32::MAX;
const COMMAND_POLL_LIMIT: u32 = u32::MAX;
const DATA_WORD_COUNT: u16 = 128;
const DMA_STREAM_INDEX: usize = 3;
const DMA_CHANNEL: u8 = 4;
const DMA_WORD_SIZE: u8 = 2;
const CMD_SET_BLOCK_LENGTH: u8 = 16;
const CMD_READ_SINGLE_BLOCK: u8 = 17;

#[repr(C, align(4))]
struct AlignedDmaWords([u32; DATA_WORD_COUNT as usize]);

// SDIO DMA cannot access the F405 CCM region where the kernel stack lives.
// Keep the transfer words in the manifest-generated DMA-visible SRAM section.
#[used]
#[unsafe(link_section = ".dma_buffer")]
static mut DMA_WORDS: AlignedDmaWords = AlignedDmaWords([0; DATA_WORD_COUNT as usize]);

/// Reads SDIO blocks without the HAL half-full FIFO tail deadlock.
pub(crate) struct RawSdioReader {
    high_capacity: bool,
}

impl RawSdioReader {
    /// Creates a reader whose card metadata is populated after HAL init.
    pub(crate) const fn new() -> Self {
        Self {
            high_capacity: false,
        }
    }
    /// Records card addressing metadata obtained from the HAL initializer.
    pub(crate) fn configure(&mut self, capacity: CardCapacity) {
        self.high_capacity = matches!(capacity, CardCapacity::HighCapacity);
    }
    /// Reads one complete 512-byte block with bounded hardware status handling.
    pub(crate) fn read_block(
        &mut self,
        address: BlockAddress,
        block: &mut Block,
    ) -> Result<(), StorageError> {
        let argument = self.command_argument(address)?;
        let registers = Self::registers();

        Self::send_command(registers, CMD_SET_BLOCK_LENGTH, BLOCK_BYTES as u32)?;
        unsafe {
            // SAFETY: The public SDIO wrapper executes this method inside a
            // critical section, so this single DMA scratch buffer has one
            // active owner for the complete transfer and copy.
            let words = &mut *core::ptr::addr_of_mut!(DMA_WORDS.0);
            Self::configure_dma(registers, words);
            Self::start_receive(registers);
            Self::start_read_command(registers, argument);
            let result = Self::receive_block(registers, words);
            Self::stop_dma();
            result?;
            for (index, word) in words.iter().enumerate() {
                let bytes = word.to_le_bytes();
                let offset = index * bytes.len();
                block[offset..offset + bytes.len()].copy_from_slice(&bytes);
            }
        }
        Ok(())
    }
    fn command_argument(&self, address: BlockAddress) -> Result<u32, StorageError> {
        if self.high_capacity {
            Ok(address.value())
        } else {
            address
                .value()
                .checked_mul(BLOCK_BYTES as u32)
                .ok_or(StorageError::InvalidBlockAddress)
        }
    }
    fn registers() -> &'static pac::sdio::RegisterBlock {
        // SAFETY: The SDIO peripheral is initialized once by the HAL and is
        // exclusively accessed by this driver while storage operations run.
        unsafe { &*pac::SDIO::ptr() }
    }
    fn send_command(
        registers: &pac::sdio::RegisterBlock,
        index: u8,
        argument: u32,
    ) -> Result<(), StorageError> {
        Self::write_command(registers, index, argument);

        let mut remaining = COMMAND_POLL_LIMIT;
        loop {
            let status = registers.sta.read();
            if status.cmdact().bit_is_clear()
                && (status.cmdrend().bit_is_set()
                    || status.ctimeout().bit_is_set()
                    || status.ccrcfail().bit_is_set())
            {
                return status_error(&status);
            }
            remaining = remaining.saturating_sub(1);
            if remaining == 0 {
                return Err(StorageError::Timeout);
            }
        }
    }
    fn start_read_command(registers: &pac::sdio::RegisterBlock, argument: u32) {
        Self::write_command(registers, CMD_READ_SINGLE_BLOCK, argument);
    }
    fn write_command(registers: &pac::sdio::RegisterBlock, index: u8, argument: u32) {
        clear_interrupts(&registers.icr);
        registers.arg.write(|writer| writer.cmdarg().bits(argument));
        registers.cmd.write(|writer| {
            writer
                .waitresp()
                .short_response()
                .cmdindex()
                .bits(index)
                .waitint()
                .disabled()
                .cpsmen()
                .enabled()
        });
    }
    fn start_receive(registers: &pac::sdio::RegisterBlock) {
        registers
            .dtimer
            .write(|writer| writer.datatime().bits(DATA_TIMEOUT_CYCLES));
        registers
            .dlen
            .write(|writer| writer.datalength().bits(BLOCK_BYTES as u32));
        registers.dctrl.write(|writer| {
            writer
                .dblocksize()
                .bits(BLOCK_SIZE_EXPONENT)
                .dtdir()
                .card_to_controller()
                .dmaen()
                .enabled()
                .dten()
                .enabled()
        });
    }
    fn configure_dma(
        registers: &pac::sdio::RegisterBlock,
        words: &mut [u32; DATA_WORD_COUNT as usize],
    ) {
        let rcc = Self::rcc();
        rcc.ahb1enr.modify(|_, writer| writer.dma2en().enabled());
        rcc.ahb1rstr.modify(|_, writer| writer.dma2rst().set_bit());
        rcc.ahb1rstr
            .modify(|_, writer| writer.dma2rst().clear_bit());
        let dma = Self::dma2();
        dma.lifcr.write(|writer| {
            writer
                .ctcif3()
                .clear()
                .chtif3()
                .clear()
                .cteif3()
                .clear()
                .cdmeif3()
                .clear()
                .cfeif3()
                .clear()
        });
        let stream = &dma.st[DMA_STREAM_INDEX];
        // SAFETY: DMA channel and word-size values are named constants within
        // the STM32F405 SDIO DMA mapping and are valid for this register.
        unsafe {
            stream.cr.write(|writer| {
                writer
                    .chsel()
                    .bits(DMA_CHANNEL)
                    .dir()
                    .peripheral_to_memory()
                    .psize()
                    .bits(DMA_WORD_SIZE)
                    .msize()
                    .bits(DMA_WORD_SIZE)
                    .minc()
                    .set_bit()
                    .pburst()
                    .incr4()
                    .mburst()
                    .incr4()
                    .pl()
                    .high()
            });
        }
        stream
            .fcr
            .write(|writer| writer.dmdis().enabled().fth().full());
        stream
            .par
            .write(|writer| unsafe { writer.pa().bits(registers.fifo.as_ptr() as u32) });
        stream
            .m0ar
            .write(|writer| unsafe { writer.m0a().bits(words.as_mut_ptr() as u32) });
        stream
            .ndtr
            .write(|writer| writer.ndt().bits(DATA_WORD_COUNT));
        stream.cr.modify(|_, writer| writer.en().set_bit());
    }

    fn receive_block(
        registers: &pac::sdio::RegisterBlock,
        words: &mut [u32; DATA_WORD_COUNT as usize],
    ) -> Result<(), StorageError> {
        loop {
            let status = registers.sta.read();
            status_error(&status)?;
            let flags = Self::dma2().lisr.read();
            if flags.teif3().bit() || flags.dmeif3().bit() || flags.feif3().bit() {
                return Err(StorageError::Transport);
            }
            if flags.tcif3().bit() {
                return Ok(());
            }
            if status.dtimeout().bit_is_set() {
                return Err(StorageError::Timeout);
            }
            if registers.dcount.read().datacount().bits() == 0 {
                return Self::finish_dma_tail(registers, words);
            }
            if status.rxact().bit_is_clear()
                && status.cmdact().bit_is_clear()
                && status.cmdrend().bit_is_set()
            {
                return Err(StorageError::Transport);
            }
        }
    }

    fn finish_dma_tail(
        registers: &pac::sdio::RegisterBlock,
        words: &mut [u32; DATA_WORD_COUNT as usize],
    ) -> Result<(), StorageError> {
        let remaining = Self::dma2().st[DMA_STREAM_INDEX].ndtr.read().ndt().bits() as usize;
        if remaining == 0 {
            return Ok(());
        }
        if remaining > words.len() || !registers.sta.read().rxdavl().bit() {
            return Err(StorageError::Transport);
        }

        Self::stop_dma();
        let offset = words.len() - remaining;
        for word in &mut words[offset..] {
            *word = registers.fifo.read().bits();
        }
        Ok(())
    }

    fn stop_dma() {
        let stream = &Self::dma2().st[DMA_STREAM_INDEX];
        stream.cr.modify(|_, writer| writer.en().clear_bit());
        while stream.cr.read().en().bit() {}
    }

    fn rcc() -> &'static pac::rcc::RegisterBlock {
        // SAFETY: RCC is a singleton peripheral accessed only for DMA2 clock setup.
        unsafe { &*pac::RCC::ptr() }
    }

    fn dma2() -> &'static pac::dma2::RegisterBlock {
        // SAFETY: DMA2 stream 3 is exclusively owned by the SDIO block reader.
        unsafe { &*pac::DMA2::ptr() }
    }
}
