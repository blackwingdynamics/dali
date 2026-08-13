//! Compile-time board selection and common board bootstrap interface.

#[cfg(feature = "board-blackpill-f411")]
mod blackpill_f411;

#[cfg(feature = "board-blackpill-f411")]
pub use blackpill_f411::{Board, HEARTBEAT_PERIOD_MS, SYSTEM_CLOCK_MHZ, initialize};

#[cfg(not(feature = "board-blackpill-f411"))]
compile_error!("Select a supported Dali OS board feature");
