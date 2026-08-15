//! Bounded SDIO block reads for the STM32F4 data path.

use crate::storage::{Block, BlockAddress, StorageError};
use stm32f4xx_hal::pac;
use stm32f4xx_hal::sdio::CardCapacity;

const BLOCK_BYTES: usize = 512;
const BLOCK_SIZE_EXPONENT: u8 = 9;
const DATA_TIMEOUT_CYCLES: u32 = u32::MAX;
const COMMAND_POLL_LIMIT: u32 = u32::MAX;
const CMD_SET_BLOCK_LENGTH: u8 = 16;
const CMD_READ_SINGLE_BLOCK: u8 = 17;

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
        Self::start_receive(registers);
        Self::send_command(registers, CMD_READ_SINGLE_BLOCK, argument)?;
        Self::receive_block(registers, block)
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
                .dten()
                .enabled()
        });
    }

    fn receive_block(
        registers: &pac::sdio::RegisterBlock,
        block: &mut Block,
    ) -> Result<(), StorageError> {
        let mut offset = 0;
        loop {
            let status = registers.sta.read();
            status_error(&status)?;

            if status.rxdavl().bit() {
                let bytes = registers.fifo.read().bits().to_le_bytes();
                let remaining = block.len() - offset;
                let count = remaining.min(bytes.len());
                block[offset..offset + count].copy_from_slice(&bytes[..count]);
                offset += count;
                if offset == block.len() {
                    break;
                }
            } else if status.rxact().bit_is_clear() {
                return Err(StorageError::Transport);
            }
        }

        loop {
            let status = registers.sta.read();
            status_error(&status)?;
            if status.rxact().bit_is_clear() || status.dataend().bit_is_set() {
                return Ok(());
            }
        }
    }
}

fn status_error(status: &pac::sdio::sta::R) -> Result<(), StorageError> {
    if status.ctimeout().bit_is_set() || status.dtimeout().bit_is_set() {
        Err(StorageError::Timeout)
    } else if status.ccrcfail().bit_is_set() || status.dcrcfail().bit_is_set() {
        Err(StorageError::DataCorruption)
    } else if status.rxoverr().bit_is_set() || status.txunderr().bit_is_set() {
        Err(StorageError::Transport)
    } else {
        Ok(())
    }
}

fn clear_interrupts(register: &pac::sdio::ICR) {
    register.write(|writer| {
        writer
            .ccrcfailc()
            .set_bit()
            .ctimeoutc()
            .set_bit()
            .ceataendc()
            .set_bit()
            .cmdrendc()
            .set_bit()
            .cmdsentc()
            .set_bit()
            .dataendc()
            .set_bit()
            .dbckendc()
            .set_bit()
            .dcrcfailc()
            .set_bit()
            .rxoverrc()
            .set_bit()
            .stbiterrc()
            .set_bit()
            .txunderrc()
            .set_bit()
            .dtimeoutc()
            .set_bit()
    });
}
