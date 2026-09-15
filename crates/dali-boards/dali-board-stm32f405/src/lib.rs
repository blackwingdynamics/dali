#![no_std]
#![warn(missing_docs)]

//! STM32F405 board backend.
//!
//! The implementation is intentionally introduced as a separate public crate.
//! The firmware composition selects this backend while the kernel consumes it
//! through the hardware-neutral `BoardBackend` contract.

mod architecture;
mod artifact;
#[cfg(feature = "storage-write")]
mod artifact_writer;
#[cfg(feature = "board-stm32f405-sd")]
mod backend;
#[cfg(feature = "abi-current")]
mod exceptions;
#[cfg(feature = "board-stm32f405-sd")]
mod logging;
#[cfg(feature = "abi-mpu")]
mod mpu;
#[cfg(any(feature = "abi-context-switch", feature = "abi-mpu"))]
mod scheduling;
mod watchdog;

pub use architecture::CortexMArchitecture;
pub use artifact::FlashArtifactReader;
#[cfg(feature = "storage-write")]
pub use artifact_writer::{FlashArtifactWriteError, FlashArtifactWriter};
pub use watchdog::{F405Watchdog, F405WatchdogError};

#[cfg(feature = "board-stm32f405-sd")]
pub use backend::{Board, Stm32f405SdioTransport};
