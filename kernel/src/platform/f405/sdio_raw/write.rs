//! Bounded CPU/FIFO block writes for the F405 SDIO peripheral.

use super::init::{Response, send};
use super::status::{clear_interrupts, status_error};
use super::{CMD_SET_BLOCK_LENGTH, CMD_WRITE_SINGLE_BLOCK, RawSdioReader};
use crate::drivers::{Block, BlockAddress, StorageError};

/// SD block size required by the raw SDIO write command.
const BLOCK_BYTES: usize = 512;
/// Hardware data timeout value used by the raw SDIO transaction.
const DATA_TIMEOUT_CYCLES: u32 = u32::MAX;
/// Maximum status polling iterations for a raw SDIO write.
const POLL_LIMIT: u32 = 1_000_000;

/// Writes one complete block through the raw SDIO command path.
pub(super) fn write_block(
    reader: &RawSdioReader,
    address: BlockAddress,
    block: &Block,
) -> Result<(), StorageError> {
    let registers = RawSdioReader::registers();
    let argument = if reader.high_capacity {
        address.value()
    } else {
        address
            .value()
            .checked_mul(BLOCK_BYTES as u32)
            .ok_or(StorageError::InvalidBlockAddress)?
    };
    send(
        registers,
        CMD_SET_BLOCK_LENGTH,
        BLOCK_BYTES as u32,
        Response::Short,
    )?;
    clear_interrupts(&registers.icr);
    registers
        .dtimer
        .write(|writer| writer.datatime().bits(DATA_TIMEOUT_CYCLES));
    registers
        .dlen
        .write(|writer| writer.datalength().bits(BLOCK_BYTES as u32));
    registers.dctrl.write(|writer| {
        writer
            .dblocksize()
            .bits(9)
            .dtdir()
            .controller_to_card()
            .dmaen()
            .disabled()
            .dten()
            .enabled()
    });
    send(registers, CMD_WRITE_SINGLE_BLOCK, argument, Response::Short)?;

    let mut offset = 0;
    for _ in 0..POLL_LIMIT {
        let status = registers.sta.read();
        status_error(&status)?;
        if offset < BLOCK_BYTES && status.txfifohe().bit_is_set() {
            for _ in 0..8 {
                let word = u32::from_le_bytes([
                    block[offset],
                    block[offset + 1],
                    block[offset + 2],
                    block[offset + 3],
                ]);
                registers.fifo.write(|writer| writer.bits(word));
                offset += 4;
            }
        }
        if offset == BLOCK_BYTES && status.txact().bit_is_clear() {
            break;
        }
    }
    if offset != BLOCK_BYTES {
        return Err(StorageError::Timeout);
    }
    for _ in 0..POLL_LIMIT {
        let status = registers.sta.read();
        status_error(&status)?;
        if status.txact().bit_is_clear() {
            return Ok(());
        }
    }
    Err(StorageError::Timeout)
}
