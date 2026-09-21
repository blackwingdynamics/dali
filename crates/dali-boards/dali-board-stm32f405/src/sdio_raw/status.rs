//! SDIO status decoding and interrupt acknowledgement helpers.

use dali_kernel_api::storage::StorageError;
use stm32f4xx_hal::pac;

/// Performs the `status error` operation for this subsystem.
///
/// Arguments select the bounded state, buffer, or hardware operation described by the signature.
///
/// # Errors
/// Returns a typed error when validation, state, or hardware access fails.
pub(super) fn status_error(status: &pac::sdio::sta::R) -> Result<(), StorageError> {
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

/// Clears the `clear interrupts` operation for this subsystem.
///
/// Arguments select the bounded state, buffer, or hardware operation described by the signature.
pub(super) fn clear_interrupts(register: &pac::sdio::ICR) {
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
