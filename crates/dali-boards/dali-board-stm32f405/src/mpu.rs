//! STM32F405 ARMv7-M MPU implementation.

mod descriptor;
mod layout;

#[cfg(feature = "abi-mpu")]
mod hardware;

#[cfg(feature = "abi-mpu")]
pub use layout::IsolationLayout;

#[cfg(feature = "abi-mpu")]
pub use hardware::{activate_application_regions, configure_hardware};
