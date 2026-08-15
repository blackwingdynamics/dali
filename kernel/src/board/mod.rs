//! Compile-time board selection and common board bootstrap interface.

#[cfg(feature = "board-blackpill-f411")]
mod blackpill_f411;

#[cfg(feature = "board-stm32f405-sd")]
mod stm32f405_sd;

#[cfg(all(feature = "board-blackpill-f411", feature = "board-stm32f405-sd"))]
compile_error!("Select exactly one Dali OS board feature");

#[cfg(all(feature = "board-blackpill-f411", feature = "sdio"))]
pub use blackpill_f411::APPLICATION_EXECUTION_SUPPORTED;
#[cfg(feature = "board-blackpill-f411")]
pub use blackpill_f411::{Board, SYSTEM_CLOCK_MHZ, initialize, set_status_led};

#[cfg(all(feature = "board-blackpill-f411", feature = "usb-cdc"))]
pub use blackpill_f411::UsbResources;

#[cfg(feature = "board-stm32f405-sd")]
pub use stm32f405_sd::APPLICATION_EXECUTION_SUPPORTED;
#[cfg(feature = "board-stm32f405-sd")]
pub use stm32f405_sd::{Board, SYSTEM_CLOCK_MHZ, SdioPins, initialize, set_status_led};

#[cfg(all(feature = "board-stm32f405-sd", feature = "usb-cdc"))]
pub use stm32f405_sd::UsbResources;

#[cfg(not(any(feature = "board-blackpill-f411", feature = "board-stm32f405-sd")))]
compile_error!("Select a supported Dali OS board feature");
