//! Platform selection boundary for hardware-specific kernel entry points.

#[cfg(feature = "board-stm32f405-sd")]
mod f405;

#[cfg(feature = "board-stm32f405-sd")]
pub(crate) use f405::{pend_usb_irq, unmask_usb_irq};

#[cfg(feature = "board-stm32f405-sd")]
use cortex_m::prelude::_embedded_hal_blocking_delay_DelayMs;

/// Stable kernel-facing operations supplied by the selected platform backend.
pub(crate) struct Platform(f405::Board);

#[cfg(feature = "board-stm32f405-sd")]
pub(crate) const SYSTEM_CLOCK_MHZ: u32 = f405::SYSTEM_CLOCK_MHZ;

#[cfg(feature = "board-stm32f405-sd")]
pub(crate) fn initialize() -> Platform {
    Platform(f405::initialize())
}

#[cfg(feature = "board-stm32f405-sd")]
impl Platform {
    pub(crate) fn set_status_led(&mut self, on: bool) {
        f405::set_status_led(&mut self.0, on);
    }

    pub(crate) fn delay_ms(&mut self, milliseconds: u32) {
        self.0.delay.delay_ms(milliseconds);
    }

    #[cfg(feature = "sdio")]
    pub(crate) fn take_sdio_reader(
        &mut self,
    ) -> Option<crate::drivers::SdioBlockReader<f405::Stm32f405SdioTransport>> {
        let (peripheral, pins, clocks) = self.0.take_sdio_resources()?;
        let transport = f405::Stm32f405SdioTransport::new(peripheral, pins, clocks);
        Some(crate::drivers::SdioBlockReader::new(transport))
    }

    #[cfg(feature = "usb-cdc")]
    pub(crate) fn take_usb_resources(&mut self) -> Option<UsbResources> {
        self.0.take_usb_resources()
    }
}

#[cfg(all(feature = "board-stm32f405-sd", feature = "abi-current"))]
pub(crate) use f405::MEMORY_PROFILE;

#[cfg(all(feature = "board-stm32f405-sd", feature = "abi-mpu"))]
pub(crate) use f405::{ISOLATION_LAYOUT, activate_application_regions};

#[cfg(all(feature = "board-stm32f405-sd", feature = "abi-mpu"))]
pub(crate) use crate::board::mpu;

#[cfg(all(feature = "board-stm32f405-sd", not(feature = "abi-current")))]
pub(crate) use f405::APPLICATION_EXECUTION_SUPPORTED;

#[cfg(all(feature = "board-stm32f405-sd", feature = "usb-cdc"))]
pub(crate) use f405::UsbResources;

#[cfg(all(feature = "board-stm32f405-sd", feature = "abi-current"))]
pub(crate) use f405::TARGET_PROFILE;

#[cfg(not(feature = "board-stm32f405-sd"))]
compile_error!("Select a supported Dali OS platform feature");
