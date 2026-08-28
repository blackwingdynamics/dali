//! Bounded SD card identification and bus setup for the F405 SDIO peripheral.

use super::{RawSdioReader, status::clear_interrupts, status::status_error};
use dali_kernel_api::storage::StorageError;
use stm32f4xx_hal::{
    pac,
    sdio::{CSD, CardCapacity, SD},
};

/// Defines the POWER SETTLE CYCLES used by this module.
const POWER_SETTLE_CYCLES: u32 = 336_000;
/// Defines the COMMAND POLL LIMIT used by this module.
const COMMAND_POLL_LIMIT: u32 = 1_000_000;
/// Defines the OCR POLL LIMIT used by this module.
const OCR_POLL_LIMIT: u32 = 1_000_000;
/// Defines the VOLTAGE WINDOW used by this module.
const VOLTAGE_WINDOW: u32 = 1 << 20;
/// Defines the OCR HIGH CAPACITY used by this module.
const OCR_HIGH_CAPACITY: u32 = 1 << 30;
/// Defines the OCR POWER UP used by this module.
const OCR_POWER_UP: u32 = 1 << 31;
/// Defines the CMD IDLE used by this module.
const CMD_IDLE: u8 = 0;
/// Defines the CMD SEND IF COND used by this module.
const CMD_SEND_IF_COND: u8 = 8;
/// Defines the CMD APP used by this module.
const CMD_APP: u8 = 55;
/// Defines the CMD SD SEND OP COND used by this module.
const CMD_SD_SEND_OP_COND: u8 = 41;
/// Defines the CMD ALL SEND CID used by this module.
const CMD_ALL_SEND_CID: u8 = 2;
/// Defines the CMD SEND RELATIVE ADDRESS used by this module.
const CMD_SEND_RELATIVE_ADDRESS: u8 = 3;
/// Defines the CMD SEND CSD used by this module.
const CMD_SEND_CSD: u8 = 9;
/// Defines the CMD SELECT CARD used by this module.
const CMD_SELECT_CARD: u8 = 7;
/// Defines the CMD SET BLOCK LENGTH used by this module.
const CMD_SET_BLOCK_LENGTH: u8 = 16;
/// Defines the CMD SET BUS WIDTH used by this module.
const CMD_SET_BUS_WIDTH: u8 = 6;
/// Defines the CMD8 ARGUMENT used by this module.
const CMD8_ARGUMENT: u32 = 0x1AA;
/// Defines the CMD8 PATTERN used by this module.
const CMD8_PATTERN: u32 = 0xAA;
/// Defines the CMD8 VOLTAGE MASK used by this module.
const CMD8_VOLTAGE_MASK: u32 = 0xF00;
/// Defines the CMD8 VOLTAGE 27 36 used by this module.
const CMD8_VOLTAGE_27_36: u32 = 0x100;
/// Defines the BUS WIDTH 4 used by this module.
const BUS_WIDTH_4: u32 = 2;
/// Defines the RCA MASK used by this module.
const RCA_MASK: u32 = 0xFFFF_0000;

#[derive(Clone, Copy)]
/// Internal Response classification for the bounded kernel path.
pub(super) enum Response {
    /// Represents the `None` response case.
    None,
    /// Represents the `Short` response case.
    Short,
    /// Represents the `Long` response case.
    Long,
}

/// Initializes the `initialize` operation for this subsystem.
///
/// Arguments select the bounded state, buffer, or hardware operation described by the signature.
///
/// # Errors
/// Returns a typed error when validation, state, or hardware access fails.
pub(super) fn initialize() -> Result<(CardCapacity, u32, u32), StorageError> {
    let registers = RawSdioReader::registers();
    power_on(registers);
    cortex_m::asm::delay(POWER_SETTLE_CYCLES);
    send(registers, CMD_IDLE, 0, Response::None)?;
    let if_cond = send(registers, CMD_SEND_IF_COND, CMD8_ARGUMENT, Response::Short)?;
    if if_cond[0] & 0xFF != CMD8_PATTERN || if_cond[0] & CMD8_VOLTAGE_MASK != CMD8_VOLTAGE_27_36 {
        return Err(StorageError::Unsupported);
    }

    let ocr = initialize_card(registers)?;
    let capacity = if ocr & OCR_HIGH_CAPACITY != 0 {
        CardCapacity::HighCapacity
    } else {
        CardCapacity::StandardCapacity
    };
    let _cid = send(registers, CMD_ALL_SEND_CID, 0, Response::Long)?;
    let rca_response = send(registers, CMD_SEND_RELATIVE_ADDRESS, 0, Response::Short)?;
    // R6 returns the RCA in bits 31:16. CMD9, CMD7, and ACMD6 use the same
    // wire representation, unlike the host-side u16 RCA helper.
    let rca = rca_response[0] & RCA_MASK;
    let response_words = send(registers, CMD_SEND_CSD, rca, Response::Long)?;
    let csd_words = [
        response_words[3],
        response_words[2],
        response_words[1],
        response_words[0],
    ];
    let csd = CSD::<SD>::from(csd_words);
    let block_count = csd.block_count();
    if block_count == 0 {
        return Err(StorageError::Unsupported);
    }
    send(registers, CMD_SELECT_CARD, rca, Response::Short)?;
    if matches!(capacity, CardCapacity::StandardCapacity) {
        send(registers, CMD_SET_BLOCK_LENGTH, 512, Response::Short)?;
    }
    send(registers, CMD_APP, rca, Response::Short)?;
    send(registers, CMD_SET_BUS_WIDTH, BUS_WIDTH_4, Response::Short)?;
    set_transfer_clock(registers);
    Ok((capacity, block_count, rca))
}

/// Powers the `power on` operation for this subsystem.
///
/// Arguments select the bounded state, buffer, or hardware operation described by the signature.
fn power_on(registers: &pac::sdio::RegisterBlock) {
    // SAFETY: 3 is the PAC-defined PWRCTRL value for SDIO power-on.
    registers
        .power
        .modify(|_, writer| unsafe { writer.pwrctrl().bits(3) });
    registers.clkcr.modify(|_, writer| {
        writer
            .clken()
            .enabled()
            .clkdiv()
            .bits(stm32f4xx_hal::sdio::ClockFreq::F400Khz as u8)
    });
}

/// Sets the `set transfer clock` operation for this subsystem.
///
/// Arguments select the bounded state, buffer, or hardware operation described by the signature.
fn set_transfer_clock(registers: &pac::sdio::RegisterBlock) {
    registers.clkcr.modify(|_, writer| {
        writer
            .clkdiv()
            .bits(stm32f4xx_hal::sdio::ClockFreq::F24Mhz as u8)
            .widbus()
            .bus_width4()
            .clken()
            .enabled()
    });
}

/// Initializes the `initialize card` operation for this subsystem.
///
/// Arguments select the bounded state, buffer, or hardware operation described by the signature.
///
/// # Errors
/// Returns a typed error when validation, state, or hardware access fails.
fn initialize_card(registers: &pac::sdio::RegisterBlock) -> Result<u32, StorageError> {
    for _ in 0..OCR_POLL_LIMIT {
        send(registers, CMD_APP, 0, Response::Short)?;
        let response = send_allow_crc(
            registers,
            CMD_SD_SEND_OP_COND,
            OCR_HIGH_CAPACITY | VOLTAGE_WINDOW,
            Response::Short,
        )?;
        if response[0] & OCR_POWER_UP != 0 {
            return Ok(response[0]);
        }
    }
    Err(StorageError::Timeout)
}

/// Sends the `send allow crc` operation for this subsystem.
///
/// Arguments select the bounded state, buffer, or hardware operation described by the signature.
fn send_allow_crc(
    registers: &pac::sdio::RegisterBlock,
    index: u8,
    argument: u32,
    response: Response,
) -> Result<[u32; 4], StorageError> {
    send_inner(registers, index, argument, response, true)
}

/// Sends the `send` operation for this subsystem.
///
/// Arguments select the bounded state, buffer, or hardware operation described by the signature.
pub(super) fn send(
    registers: &pac::sdio::RegisterBlock,
    index: u8,
    argument: u32,
    response: Response,
) -> Result<[u32; 4], StorageError> {
    send_inner(registers, index, argument, response, false)
}

/// Sends the `send inner` operation for this subsystem.
///
/// Arguments select the bounded state, buffer, or hardware operation described by the signature.
fn send_inner(
    registers: &pac::sdio::RegisterBlock,
    index: u8,
    argument: u32,
    response: Response,
    allow_crc_error: bool,
) -> Result<[u32; 4], StorageError> {
    clear_interrupts(&registers.icr);
    registers.arg.write(|writer| writer.cmdarg().bits(argument));
    registers.cmd.write(|writer| {
        writer
            .cmdindex()
            .bits(index)
            .waitint()
            .disabled()
            .cpsmen()
            .enabled();
        match response {
            Response::None => writer.waitresp().no_response(),
            Response::Short => writer.waitresp().short_response(),
            Response::Long => writer.waitresp().long_response(),
        }
    });
    for _ in 0..COMMAND_POLL_LIMIT {
        let status = registers.sta.read();
        let complete = if matches!(response, Response::None) {
            status.cmdsent().bit_is_set()
        } else {
            status.cmdrend().bit_is_set() || status.ccrcfail().bit_is_set()
        };
        if status.cmdact().bit_is_clear() && (complete || status.ctimeout().bit_is_set()) {
            if (!allow_crc_error || !status.ccrcfail().bit_is_set())
                && let Err(error) = status_error(&status)
            {
                report_command_failure(index, error);
                return Err(error);
            }
            return Ok([
                registers.resp1.read().bits(),
                registers.resp2.read().bits(),
                registers.resp3.read().bits(),
                registers.resp4.read().bits(),
            ]);
        }
    }
    crate::logging::error(
        crate::logging::BOOT_SUBSYSTEM,
        format_args!("[SDIO] CMD{} timed out", index),
    );
    Err(StorageError::Timeout)
}

/// Reports the `report command failure` operation for this subsystem.
///
/// Arguments select the bounded state, buffer, or hardware operation described by the signature.
fn report_command_failure(index: u8, error: StorageError) {
    if matches!(error, StorageError::NotReady | StorageError::Timeout) {
        return;
    }
    crate::logging::error(
        crate::logging::BOOT_SUBSYSTEM,
        format_args!("[SDIO] CMD{} failed: {:?}", index, error),
    );
}
