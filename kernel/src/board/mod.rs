//! Compile-time board selection and common board bootstrap interface.

pub mod mpu;

#[cfg(feature = "board-stm32f405-sd")]
mod stm32f405_sd;

#[cfg(all(feature = "board-stm32f405-sd", not(feature = "abi-v3")))]
pub use stm32f405_sd::APPLICATION_EXECUTION_SUPPORTED;
#[cfg(all(feature = "board-stm32f405-sd", feature = "abi-v3-mpu"))]
pub use stm32f405_sd::ISOLATION_LAYOUT;
#[cfg(feature = "board-stm32f405-sd")]
pub use stm32f405_sd::{Board, SYSTEM_CLOCK_MHZ, SdioPins, initialize, set_status_led};

#[cfg(all(feature = "board-stm32f405-sd", feature = "usb-cdc"))]
pub use stm32f405_sd::UsbResources;

#[cfg(not(feature = "board-stm32f405-sd"))]
compile_error!("Select a supported Dali OS board feature");
