#![no_std]
#![warn(missing_docs)]

//! STM32F405 board backend.
//!
//! The implementation is intentionally introduced as a separate public crate.
//! Existing kernel F405 code is migrated here in subsequent atomic checkpoints;
//! this first checkpoint establishes the publication and dependency boundary.

#[cfg(feature = "board-stm32f405-sd")]
mod backend;
#[cfg(feature = "board-stm32f405-sd")]
mod logging;
#[cfg(feature = "abi-mpu")]
mod mpu;
mod watchdog;

pub use watchdog::{F405Watchdog, F405WatchdogError};

#[cfg(feature = "board-stm32f405-sd")]
pub use backend::{Board, Stm32f405SdioTransport};
