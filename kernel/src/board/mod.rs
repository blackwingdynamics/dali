//! Compile-time board selection and common board bootstrap interface.

#[cfg(feature = "board-blackpill-f411")]
mod blackpill_f411;

#[cfg(feature = "board-stm32f405-sd")]
mod stm32f405_sd;

#[cfg(all(feature = "board-blackpill-f411", feature = "board-stm32f405-sd"))]
compile_error!("Select exactly one Dali OS board feature");

#[cfg(feature = "board-blackpill-f411")]
pub use blackpill_f411::{Board, HEARTBEAT_PERIOD_MS, SYSTEM_CLOCK_MHZ, initialize};

#[cfg(feature = "board-stm32f405-sd")]
pub use stm32f405_sd::{Board, HEARTBEAT_PERIOD_MS, SYSTEM_CLOCK_MHZ, SdioPins, initialize};

#[cfg(not(any(feature = "board-blackpill-f411", feature = "board-stm32f405-sd")))]
compile_error!("Select a supported Dali OS board feature");
