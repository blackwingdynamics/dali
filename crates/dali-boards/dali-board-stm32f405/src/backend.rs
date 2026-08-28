//! STM32F405 target selection and interrupt bindings.

#[cfg(feature = "board-stm32f405-sd")]
#[path = "board.rs"]
mod board;
#[path = "drivers/mod.rs"]
pub(crate) mod drivers;
#[cfg(all(feature = "board-stm32f405-sd", feature = "sdio"))]
#[path = "sdio.rs"]
mod sdio;
#[cfg(all(feature = "board-stm32f405-sd", feature = "sdio"))]
#[path = "sdio_raw/mod.rs"]
mod sdio_raw;

#[cfg(feature = "board-stm32f405-sd")]
pub use board::Board;
#[cfg(all(feature = "board-stm32f405-sd", feature = "abi-current"))]
pub use board::MEMORY_PROFILE;
#[cfg(all(feature = "board-stm32f405-sd", feature = "abi-mpu"))]
pub use board::{ISOLATION_LAYOUT, activate_application_regions};
#[cfg(all(feature = "board-stm32f405-sd", feature = "sdio"))]
pub use sdio::Stm32f405SdioTransport;

#[cfg(feature = "board-stm32f405-sd")]
impl dali_kernel_api::BoardBackend for board::Board {
    type Architecture = crate::CortexMArchitecture;
    type StatusLed = board::StatusLed;
    type UserKey = board::UserKey;
    type Watchdog = crate::F405Watchdog;
    type StorageReader = sdio::SdioBlockReader;

    fn memory_protection_operations() -> Option<dali_kernel_api::MemoryProtectionOperations> {
        #[cfg(feature = "abi-mpu")]
        {
            Some(dali_kernel_api::MemoryProtectionOperations {
                configure: board::configure_memory_protection,
                activate_application_regions: board::activate_application_regions,
            })
        }
        #[cfg(not(feature = "abi-mpu"))]
        {
            None
        }
    }

    fn info() -> dali_kernel_api::BoardInfo {
        dali_kernel_api::BoardInfo {
            backend: "stm32f405",
            target: &dali_targets::TARGET_F405,
            capabilities: dali_targets::TARGET_F405.capabilities,
            memory: dali_targets::TARGET_F405.memory,
        }
    }

    fn system_clock_mhz() -> u32 {
        board::SYSTEM_CLOCK_MHZ
    }

    fn initialize() -> Result<Self, dali_kernel_api::BoardError> {
        Ok(board::initialize())
    }

    fn set_status_led(&mut self, on: bool) -> Result<(), dali_kernel_api::BoardError> {
        board::set_status_led(self, on);
        Ok(())
    }

    fn delay_ms(&mut self, milliseconds: u32) -> Result<(), dali_kernel_api::BoardError> {
        board::delay_ms(self, milliseconds);
        Ok(())
    }

    fn poll_user_key(&mut self) {
        board::Board::poll_user_key(self);
    }

    fn take_watchdog(&mut self) -> Result<Self::Watchdog, dali_kernel_api::BoardError> {
        self.take_watchdog()
            .ok_or(dali_kernel_api::BoardError::ResourceUnavailable)
    }

    fn take_storage_reader(&mut self) -> Result<Self::StorageReader, dali_kernel_api::BoardError> {
        let (peripheral, pins, clocks) = self
            .take_sdio_resources()
            .ok_or(dali_kernel_api::BoardError::ResourceUnavailable)?;
        Ok(sdio::SdioBlockReader::new(Stm32f405SdioTransport::new(
            peripheral, pins, clocks,
        )))
    }

    fn reset_cause(&self) -> dali_kernel_api::ResetCause {
        self.reset_cause()
    }

    fn application_execution_supported() -> bool {
        true
    }
}

#[cfg(feature = "abi-current")]
use dali_targets::TargetProfile;

#[cfg(feature = "abi-current")]
/// Target profile used by the feature-gated application loader.
pub(crate) const TARGET_PROFILE: &TargetProfile = &dali_targets::TARGET_F405;

#[cfg(all(feature = "abi-context-switch", feature = "abi-current"))]
/// Maximum number of application contexts supported by the target profile.
pub(crate) const CONTEXT_CAPACITY: usize = dali_targets::TARGET_F405_CONTEXT_CAPACITY;

#[cfg(all(feature = "abi-context-switch", feature = "abi-current"))]
/// Scheduler configuration selected by the target manifest.
pub(crate) const SCHEDULER_PROFILE: Option<dali_targets::SchedulerProfile> =
    dali_targets::TARGET_F405.scheduler;

#[cfg(feature = "driver-hardware-test")]
use stm32f4xx_hal::pac;
#[cfg(any(feature = "usb-cdc", feature = "driver-hardware-test"))]
use stm32f4xx_hal::pac::interrupt;

#[cfg(feature = "driver-hardware-test")]
const USER_KEY_EXTI_MASK: u32 = 1_u32 << 13;

/// Enables the shared NVIC line used by EXTI10..EXTI15 during the hardware probe.
#[cfg(feature = "driver-hardware-test")]
pub(crate) fn unmask_exti15_10_irq() {
    // SAFETY: The board configured only the kernel-owned PC13 EXTI line before
    // enabling this shared interrupt during the bounded driver probe.
    unsafe { cortex_m::peripheral::NVIC::unmask(pac::Interrupt::EXTI15_10) };
}

/// Handles the board-owned PC13 falling-edge event without waiting for storage.
#[cfg(feature = "driver-hardware-test")]
#[interrupt]
fn EXTI15_10() {
    let exti = unsafe {
        // SAFETY: EXTI is exclusively owned by the board's PC13 driver in the
        // hardware-test build, and this handler only accesses EXTI13's bit.
        &*pac::EXTI::ptr()
    };
    if exti.pr.read().bits() & USER_KEY_EXTI_MASK != 0 {
        exti.pr.write(|writer| unsafe {
            // SAFETY: USER_KEY_EXTI_MASK names only EXTI13; STM32 EXTI PR is
            // write-one-to-clear.
            writer.bits(USER_KEY_EXTI_MASK)
        });
        crate::logging::info(
            crate::logging::BOOT_SUBSYSTEM,
            format_args!("[DRIVER][EXTI] PC13 interrupt triggered"),
        );
    }
}
