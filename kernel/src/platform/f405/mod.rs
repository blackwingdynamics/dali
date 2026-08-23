//! STM32F405 target selection and interrupt bindings.

#[cfg(feature = "board-stm32f405-sd")]
mod board;
mod drivers;
#[cfg(all(feature = "board-stm32f405-sd", feature = "sdio"))]
mod sdio;
#[cfg(all(feature = "board-stm32f405-sd", feature = "sdio"))]
mod sdio_raw;
mod watchdog;

#[cfg(all(feature = "board-stm32f405-sd", not(feature = "abi-current")))]
pub(crate) use board::APPLICATION_EXECUTION_SUPPORTED;
#[cfg(feature = "board-stm32f405-sd")]
pub(crate) use board::Board;
#[cfg(all(feature = "board-stm32f405-sd", feature = "abi-current"))]
pub(crate) use board::MEMORY_PROFILE;
#[cfg(all(feature = "board-stm32f405-sd", feature = "abi-mpu"))]
pub(crate) use board::{ISOLATION_LAYOUT, activate_application_regions};
#[cfg(all(feature = "board-stm32f405-sd", feature = "sdio"))]
pub(crate) use sdio::Stm32f405SdioTransport;
pub(crate) use watchdog::F405Watchdog;

#[cfg(feature = "board-stm32f405-sd")]
impl crate::platform::Backend for board::Board {
    const SYSTEM_CLOCK_MHZ: u32 = board::SYSTEM_CLOCK_MHZ;

    #[cfg(feature = "sdio")]
    type SdioReader = crate::drivers::SdioBlockReader<Stm32f405SdioTransport>;

    #[cfg(feature = "usb-cdc")]
    type UsbResources = board::UsbResources;

    fn initialize() -> Self {
        board::initialize()
    }

    fn set_status_led(&mut self, on: bool) {
        board::set_status_led(self, on);
    }

    fn delay_ms(&mut self, milliseconds: u32) {
        board::delay_ms(self, milliseconds);
    }

    #[cfg(feature = "abi-context-switch")]
    fn enable_scheduler_tick(&mut self, tick_hz: u32) -> bool {
        board::enable_scheduler_tick(self, tick_hz)
    }

    #[cfg(feature = "sdio")]
    fn take_sdio_reader(&mut self) -> Option<Self::SdioReader> {
        let (peripheral, pins, clocks) = self.take_sdio_resources()?;
        let transport = Stm32f405SdioTransport::new(peripheral, pins, clocks);
        Some(crate::drivers::SdioBlockReader::new(transport))
    }

    #[cfg(feature = "usb-cdc")]
    fn take_usb_resources(&mut self) -> Option<Self::UsbResources> {
        self.take_usb_resources()
    }

    #[cfg(feature = "usb-cdc")]
    fn unmask_usb_irq() {
        board::unmask_usb_irq();
    }

    #[cfg(feature = "usb-cdc")]
    fn pend_usb_irq() {
        board::pend_usb_irq();
    }
}

#[cfg(feature = "abi-current")]
use dali_targets::TargetProfile;

#[cfg(feature = "abi-current")]
pub(crate) const TARGET_PROFILE: &TargetProfile = &dali_targets::TARGET_F405;

/// DMA-visible region reserved by the target manifest for kernel transport buffers.
pub(crate) const DMA_REGION: dali_targets::TargetMemoryRegion =
    dali_targets::TARGET_F405.memory.dma;

#[cfg(all(feature = "abi-context-switch", feature = "abi-current"))]
pub(crate) const CONTEXT_CAPACITY: usize = dali_targets::TARGET_F405_CONTEXT_CAPACITY;

#[cfg(all(feature = "abi-context-switch", feature = "abi-current"))]
pub(crate) const SCHEDULER_PROFILE: Option<dali_targets::SchedulerProfile> =
    dali_targets::TARGET_F405.scheduler;

#[cfg(feature = "usb-cdc")]
use stm32f4xx_hal::pac::interrupt;

#[cfg(feature = "usb-cdc")]
#[interrupt]
fn OTG_FS() {
    crate::logging::service_usb_irq();
}
