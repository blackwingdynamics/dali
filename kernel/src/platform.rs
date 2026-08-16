//! Platform selection boundary for hardware-specific kernel entry points.

#[cfg(feature = "board-stm32f405-sd")]
mod stm32f405;
#[cfg(feature = "board-stm32f405-sd")]
mod stm32f405_board;

#[cfg(feature = "board-stm32f405-sd")]
pub(crate) use stm32f405_board::{Board, SYSTEM_CLOCK_MHZ, SdioPins, initialize, set_status_led};

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
