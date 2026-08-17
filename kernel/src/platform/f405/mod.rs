//! STM32F405 target selection and interrupt bindings.

#[cfg(feature = "board-stm32f405-sd")]
mod board;
#[cfg(all(feature = "board-stm32f405-sd", feature = "sdio"))]
mod sdio;
#[cfg(all(feature = "board-stm32f405-sd", feature = "sdio"))]
mod sdio_raw;

#[cfg(all(feature = "board-stm32f405-sd", not(feature = "abi-current")))]
pub(crate) use board::APPLICATION_EXECUTION_SUPPORTED;
#[cfg(all(feature = "board-stm32f405-sd", feature = "abi-current"))]
pub(crate) use board::MEMORY_PROFILE;
#[cfg(all(feature = "board-stm32f405-sd", feature = "usb-cdc"))]
pub(crate) use board::UsbResources;
#[cfg(feature = "board-stm32f405-sd")]
pub(crate) use board::{
    Board, SYSTEM_CLOCK_MHZ, initialize, pend_usb_irq, set_status_led, unmask_usb_irq,
};
#[cfg(all(feature = "board-stm32f405-sd", feature = "abi-mpu"))]
pub(crate) use board::{ISOLATION_LAYOUT, activate_application_regions};
#[cfg(all(feature = "board-stm32f405-sd", feature = "sdio"))]
pub(crate) use sdio::Stm32f405SdioTransport;

#[cfg(feature = "abi-current")]
use dali_targets::TargetProfile;

#[cfg(feature = "abi-current")]
pub(crate) const TARGET_PROFILE: &TargetProfile = &dali_targets::TARGET_F405;

#[cfg(feature = "usb-cdc")]
use stm32f4xx_hal::pac::interrupt;

#[cfg(feature = "usb-cdc")]
#[interrupt]
fn OTG_FS() {
    crate::logging::service_usb_irq();
}
