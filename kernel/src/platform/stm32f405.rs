//! STM32F405 target selection and interrupt bindings.

#[cfg(feature = "abi-v3")]
use dali_targets::TargetProfile;

#[cfg(feature = "abi-v3")]
pub(crate) const TARGET_PROFILE: &TargetProfile = &dali_targets::TARGET_F405;

#[cfg(feature = "usb-cdc")]
use stm32f4xx_hal::pac::interrupt;

#[cfg(feature = "usb-cdc")]
#[interrupt]
fn OTG_FS() {
    crate::logging::service_usb_irq();
}
