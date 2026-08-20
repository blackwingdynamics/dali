//! SDIO status decoding and interrupt acknowledgement helpers.

use crate::drivers::StorageError;
use stm32f4xx_hal::pac;

pub(super) fn status_error(status: &pac::sdio::sta::R) -> Result<(), StorageError> {
    if status.ctimeout().bit_is_set() || status.dtimeout().bit_is_set() {
        Err(StorageError::Timeout)
    } else if status.ccrcfail().bit_is_set() || status.dcrcfail().bit_is_set() {
        Err(StorageError::DataCorruption)
    } else if status.rxoverr().bit_is_set() {
        Err(StorageError::SdioStatusFailure(status.bits()))
    } else if status.txunderr().bit_is_set() {
        Err(StorageError::SdioTransmitUnderrun(status.bits()))
    } else {
        Ok(())
    }
}

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
