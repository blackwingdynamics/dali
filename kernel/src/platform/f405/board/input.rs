//! Board-local EXTI polling for the documented user-key input.

use super::Board;
use dali_driver_api::DriverError;
#[cfg(not(feature = "driver-hardware-test"))]
use dali_driver_api::InterruptPin;

impl Board {
    /// Polls the enabled user-key EXTI line without taking interrupt ownership.
    pub(crate) fn poll_user_key(&mut self) {
        if !self.user_key_enabled {
            return;
        }
        #[cfg(feature = "driver-hardware-test")]
        let pending = self.user_key.poll_user_key_trigger();
        #[cfg(not(feature = "driver-hardware-test"))]
        let pending = self.user_key.take_pending();
        match pending {
            Ok(true) => {
                #[cfg(feature = "driver-hardware-test")]
                crate::logging::info(
                    crate::logging::BOOT_SUBSYSTEM,
                    format_args!("[DRIVER][EXTI] User-key interrupt triggered"),
                );
            }
            Ok(false) | Err(DriverError::InvalidState) => {}
            Err(error) => crate::logging::error(
                crate::logging::BOOT_SUBSYSTEM,
                format_args!("[DRIVER][EXTI] User-key poll failed: {:?}", error),
            ),
        }
        #[cfg(feature = "driver-hardware-test")]
        super::super::unmask_exti15_10_irq();
    }
}
