//! Platform selection boundary for hardware-specific kernel entry points.

#[cfg(feature = "board-stm32f405-sd")]
mod stm32f405;

#[cfg(feature = "board-stm32f405-sd")]
pub(crate) use crate::board::{Board, SYSTEM_CLOCK_MHZ, SdioPins, initialize, set_status_led};

#[cfg(all(feature = "board-stm32f405-sd", feature = "abi-v3"))]
pub(crate) use crate::board::MEMORY_PROFILE;

#[cfg(all(feature = "board-stm32f405-sd", feature = "abi-v3-mpu"))]
pub(crate) use crate::board::{ISOLATION_LAYOUT, activate_application_regions};

#[cfg(all(feature = "board-stm32f405-sd", feature = "abi-v3-mpu"))]
pub(crate) use crate::board::mpu;

#[cfg(all(feature = "board-stm32f405-sd", not(feature = "abi-v3")))]
pub(crate) use crate::board::APPLICATION_EXECUTION_SUPPORTED;

#[cfg(all(feature = "board-stm32f405-sd", feature = "usb-cdc"))]
pub(crate) use crate::board::UsbResources;

#[cfg(all(feature = "board-stm32f405-sd", feature = "abi-v3"))]
pub(crate) use stm32f405::TARGET_PROFILE;

#[cfg(not(feature = "board-stm32f405-sd"))]
compile_error!("Select a supported Dali OS platform feature");
