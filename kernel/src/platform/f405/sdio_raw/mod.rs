//! Bounded SDIO block reads for the STM32F4 data path.

use crate::drivers::{Block, BlockAddress, StorageError};
use crate::runtime::memory::{DmaBuffer, dma::CURRENT_POLICY, dma::DmaOwner};
use stm32f4xx_hal::pac;
use stm32f4xx_hal::sdio::CardCapacity;

/// Internal dma helpers for the surrounding subsystem.
mod dma;
mod init;
mod status;
#[cfg(feature = "storage-write")]
mod write;
use status::{clear_interrupts, status_error};

/// Defines the BLOCK BYTES used by this module.
const BLOCK_BYTES: usize = 512;
/// Defines the BLOCK SIZE EXPONENT used by this module.
const BLOCK_SIZE_EXPONENT: u8 = 9;
/// Defines the DATA TIMEOUT CYCLES used by this module.
const DATA_TIMEOUT_CYCLES: u32 = u32::MAX;
/// Defines the COMMAND POLL LIMIT used by this module.
const COMMAND_POLL_LIMIT: u32 = u32::MAX;
/// Defines the DATA WORD COUNT used by this module.
const DATA_WORD_COUNT: u16 = 128;
/// Defines the DMA STREAM INDEX used by this module.
const DMA_STREAM_INDEX: usize = 3;
/// Defines the DMA CHANNEL used by this module.
const DMA_CHANNEL: u8 = 4;
/// Defines the DMA WORD SIZE used by this module.
const DMA_WORD_SIZE: u8 = 2;
/// Defines the CMD SET BLOCK LENGTH used by this module.
const CMD_SET_BLOCK_LENGTH: u8 = 16;
/// Defines the CMD READ SINGLE BLOCK used by this module.
const CMD_READ_SINGLE_BLOCK: u8 = 17;
#[cfg(feature = "storage-write")]
/// SD command number for writing one block.
const CMD_WRITE_SINGLE_BLOCK: u8 = 24;
#[cfg(feature = "storage-write")]
/// SD command number for reading card status.
const CMD_SEND_STATUS: u8 = 13;
#[cfg(feature = "storage-write")]
/// Card-status flag indicating readiness for another command.
const CARD_READY_FOR_DATA: u32 = 1 << 8;
#[cfg(feature = "storage-write")]
/// Card state value representing the transfer state.
const CARD_STATE_TRAN: u32 = 4;
#[cfg(feature = "storage-write")]
/// Mask selecting the card state field in the status response.
const CARD_STATE_MASK: u32 = 0xF << 9;
#[cfg(feature = "storage-write")]
/// Maximum card-status polling iterations.
const CARD_STATUS_POLL_LIMIT: u32 = 1_000_000;

#[repr(C, align(4))]
/// Internal AlignedDmaWords record used by the bounded kernel path.
struct AlignedDmaWords([u32; DATA_WORD_COUNT as usize]);

// SDIO DMA cannot access the F405 CCM region where the kernel stack lives.
// Keep the transfer words in the manifest-generated DMA-visible SRAM section.
#[used]
#[unsafe(link_section = ".dma_buffer")]
/// Kernel-owned static storage for DMA WORDS.
static mut DMA_WORDS: AlignedDmaWords = AlignedDmaWords([0; DATA_WORD_COUNT as usize]);

#[cfg(feature = "dma-test-fixture")]
#[used]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".dma_buffer")]
static mut DMA_TRACE_MARKER: u32 = 0;

#[cfg(feature = "dma-test-fixture")]
#[repr(C)]
#[derive(Clone, Copy)]
struct DmaTraceSnapshot {
    dma_cr: u32,
    dma_ndtr: u32,
    dma_par: u32,
    dma_m0ar: u32,
    sdio_dctrl: u32,
    sdio_dcount: u32,
    sdio_sta: u32,
    sdio_fifocnt: u32,
}

#[cfg(feature = "dma-test-fixture")]
#[used]
#[unsafe(link_section = ".dma_buffer")]
static mut DMA_TRACE_SNAPSHOT: DmaTraceSnapshot = DmaTraceSnapshot {
    dma_cr: 0,
    dma_ndtr: 0,
    dma_par: 0,
    dma_m0ar: 0,
    sdio_dctrl: 0,
    sdio_dcount: 0,
    sdio_sta: 0,
    sdio_fifocnt: 0,
};

#[cfg(feature = "dma-test-fixture")]
const DMA_TRACE_ACTIVE: u32 = 0xD1A0_0001;
#[cfg(feature = "dma-test-fixture")]
const DMA_TRACE_COMPLETE: u32 = 0xD1A0_0002;

/// Reads SDIO blocks without the HAL half-full FIFO tail deadlock.
pub(crate) struct RawSdioReader {
    /// Stores the high capacity associated with this bounded state.
    high_capacity: bool,
    /// Stores the relative address associated with this bounded state.
    relative_address: u32,
}

impl RawSdioReader {
    /// Creates a reader whose card metadata is populated after HAL init.
    pub(crate) const fn new() -> Self {
        Self {
            high_capacity: false,
            relative_address: 0,
        }
    }
    /// Records card addressing metadata obtained from the HAL initializer.
    pub(crate) fn configure(&mut self, capacity: CardCapacity, relative_address: u32) {
        self.high_capacity = matches!(capacity, CardCapacity::HighCapacity);
        self.relative_address = relative_address;
    }

    /// Initializes an SD card through bounded F405 SDIO command polling.
    pub(crate) fn initialize(&mut self) -> Result<u32, StorageError> {
        let (capacity, block_count, relative_address) = init::initialize()?;
        self.configure(capacity, relative_address);
        Ok(block_count)
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
            CURRENT_POLICY
                .authorize(DmaOwner::Kernel)
                .map_err(|_| StorageError::Transport)?;
            let words = &mut *core::ptr::addr_of_mut!(DMA_WORDS.0);
            let mut dma_words = DmaBuffer::new(words, crate::platform::DMA_REGION)
                .map_err(|_| StorageError::Transport)?;
            let words = dma_words.as_mut_slice();
            Self::configure_dma(registers, words);
            Self::start_receive(registers);
            Self::start_read_command(registers, argument);
            #[cfg(feature = "dma-test-fixture")]
            let dma = &*pac::DMA2::ptr();
            #[cfg(feature = "dma-test-fixture")]
            // SAFETY: The fixture snapshot is kernel-owned diagnostic state
            // written inside the exclusive transfer critical section.
            core::ptr::write_volatile(
                core::ptr::addr_of_mut!(DMA_TRACE_SNAPSHOT),
                DmaTraceSnapshot {
                    dma_cr: dma.st[DMA_STREAM_INDEX].cr.read().bits(),
                    dma_ndtr: dma.st[DMA_STREAM_INDEX].ndtr.read().bits(),
                    dma_par: dma.st[DMA_STREAM_INDEX].par.read().bits(),
                    dma_m0ar: dma.st[DMA_STREAM_INDEX].m0ar.read().bits(),
                    sdio_dctrl: registers.dctrl.read().bits(),
                    sdio_dcount: registers.dcount.read().bits(),
                    sdio_sta: registers.sta.read().bits(),
                    sdio_fifocnt: registers.fifocnt.read().bits(),
                },
            );
            #[cfg(feature = "dma-test-fixture")]
            // SAFETY: The fixture marker is kernel-owned diagnostic state
            // written inside the same exclusive transfer critical section.
            core::ptr::write_volatile(core::ptr::addr_of_mut!(DMA_TRACE_MARKER), DMA_TRACE_ACTIVE);
            let result = Self::receive_block(registers, words);
            Self::stop_dma();
            #[cfg(feature = "dma-test-fixture")]
            // SAFETY: The marker remains kernel-owned and the transfer has
            // stopped before the completion state is published.
            core::ptr::write_volatile(
                core::ptr::addr_of_mut!(DMA_TRACE_MARKER),
                DMA_TRACE_COMPLETE,
            );
            result?;
            for (index, word) in words.iter().enumerate() {
                let bytes = word.to_le_bytes();
                let offset = index * bytes.len();
                block[offset..offset + bytes.len()].copy_from_slice(&bytes);
            }
        }
        Ok(())
    }

    /// Writes one block through the bounded CPU/FIFO SDIO path.
    #[cfg(feature = "storage-write")]
    pub(crate) fn write_block(
        &mut self,
        address: BlockAddress,
        block: &Block,
    ) -> Result<(), StorageError> {
        write::write_block(self, address, block)
    }

    /// Waits until the card reports that all writes are durable and idle.
    #[cfg(feature = "storage-write")]
    pub(crate) fn flush(&self) -> Result<(), StorageError> {
        let registers = Self::registers();
        for _ in 0..CARD_STATUS_POLL_LIMIT {
            let response = init::send(
                registers,
                CMD_SEND_STATUS,
                self.relative_address,
                init::Response::Short,
            )?;
            let status = response[0];
            if status & CARD_READY_FOR_DATA != 0
                && (status & CARD_STATE_MASK) >> 9 == CARD_STATE_TRAN
            {
                return Ok(());
            }
        }
        Err(StorageError::Timeout)
    }
    /// Builds the `command argument` operation for this subsystem.
    ///
    /// Arguments select the bounded state, buffer, or hardware operation described by the signature.
    ///
    /// # Errors
    /// Returns a typed error when validation, state, or hardware access fails.
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
    /// Performs the `registers` operation for this subsystem.
    ///
    /// Arguments select the bounded state, buffer, or hardware operation described by the signature.
    fn registers() -> &'static pac::sdio::RegisterBlock {
        // SAFETY: The SDIO peripheral is initialized once by the HAL and is
        // exclusively accessed by this driver while storage operations run.
        unsafe { &*pac::SDIO::ptr() }
    }
    /// Sends the `send command` operation for this subsystem.
    ///
    /// Arguments select the bounded state, buffer, or hardware operation described by the signature.
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
    /// Starts the `start read command` operation for this subsystem.
    ///
    /// Arguments select the bounded state, buffer, or hardware operation described by the signature.
    fn start_read_command(registers: &pac::sdio::RegisterBlock, argument: u32) {
        Self::write_command(registers, CMD_READ_SINGLE_BLOCK, argument);
    }
    /// Writes the `write command` operation for this subsystem.
    ///
    /// Arguments select the bounded state, buffer, or hardware operation described by the signature.
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
    /// Starts the `start receive` operation for this subsystem.
    ///
    /// Arguments select the bounded state, buffer, or hardware operation described by the signature.
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
}
