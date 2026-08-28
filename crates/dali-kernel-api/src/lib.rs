#![no_std]
#![warn(missing_docs)]

//! Public, hardware-neutral contracts for integrating Dali OS backends.

pub mod architecture;
pub mod board;
pub mod dma;
pub mod storage;
#[cfg(feature = "usb-cdc")]
pub mod usb;

pub use architecture::{ArchitectureBackend, ArchitectureOperations};
pub use board::{
    BoardBackend, BoardError, BoardInfo, BoardServices, MemoryProtectionOperations, ResetCause,
    WatchdogBackend,
};
#[cfg(feature = "usb-cdc")]
pub use usb::{UsbBusReset, UsbResetDelay, UsbResources};
