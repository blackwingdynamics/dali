//! Platform selection boundary for hardware-specific kernel entry points.

#[cfg(feature = "board-stm32f405-sd")]
mod stm32f405;
#[cfg(feature = "board-stm32f405-sd")]
mod stm32f405_board;
#[cfg(all(feature = "board-stm32f405-sd", feature = "sdio"))]
mod stm32f405_sdio;
#[cfg(all(feature = "board-stm32f405-sd", feature = "sdio"))]
mod stm32f405_sdio_raw;

#[cfg(feature = "board-stm32f405-sd")]
pub(crate) use stm32f405_board::{pend_usb_irq, unmask_usb_irq};

#[cfg(feature = "board-stm32f405-sd")]
use cortex_m::prelude::_embedded_hal_blocking_delay_DelayMs;

/// Stable kernel-facing operations supplied by the selected platform backend.
pub(crate) struct Platform(stm32f405_board::Board);

#[cfg(feature = "board-stm32f405-sd")]
pub(crate) const SYSTEM_CLOCK_MHZ: u32 = stm32f405_board::SYSTEM_CLOCK_MHZ;

#[cfg(feature = "board-stm32f405-sd")]
pub(crate) fn initialize() -> Platform {
    Platform(stm32f405_board::initialize())
}

#[cfg(feature = "board-stm32f405-sd")]
impl Platform {
    pub(crate) fn set_status_led(&mut self, on: bool) {
        stm32f405_board::set_status_led(&mut self.0, on);
    }

    pub(crate) fn delay_ms(&mut self, milliseconds: u32) {
        self.0.delay.delay_ms(milliseconds);
    }

    #[cfg(feature = "sdio")]
    pub(crate) fn take_sdio_reader(&mut self) -> Option<SdioBlockReader> {
        let (peripheral, pins, clocks) = self.0.take_sdio_resources()?;
        Some(SdioBlockReader::new(peripheral, pins, clocks))
    }

    #[cfg(feature = "usb-cdc")]
    pub(crate) fn take_usb_resources(&mut self) -> Option<UsbResources> {
        self.0.take_usb_resources()
    }
}

#[cfg(all(feature = "board-stm32f405-sd", feature = "sdio"))]
pub(crate) use stm32f405_sdio::SdioBlockReader;

#[cfg(all(feature = "board-stm32f405-sd", feature = "abi-current"))]
pub(crate) use stm32f405_board::MEMORY_PROFILE;

#[cfg(all(feature = "board-stm32f405-sd", feature = "abi-mpu"))]
pub(crate) use stm32f405_board::{ISOLATION_LAYOUT, activate_application_regions};

#[cfg(all(feature = "board-stm32f405-sd", feature = "abi-mpu"))]
pub(crate) use crate::board::mpu;

#[cfg(all(feature = "board-stm32f405-sd", not(feature = "abi-current")))]
pub(crate) use stm32f405_board::APPLICATION_EXECUTION_SUPPORTED;

#[cfg(all(feature = "board-stm32f405-sd", feature = "usb-cdc"))]
pub(crate) use stm32f405_board::UsbResources;

#[cfg(all(feature = "board-stm32f405-sd", feature = "abi-current"))]
pub(crate) use stm32f405::TARGET_PROFILE;

#[cfg(not(feature = "board-stm32f405-sd"))]
compile_error!("Select a supported Dali OS platform feature");
