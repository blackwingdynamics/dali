#![no_std]
#![warn(missing_docs)]

//! Public, hardware-neutral contracts for integrating Dali OS backends.

pub mod architecture;
pub mod board;
pub mod dma;
pub mod installation;
pub mod storage;
#[cfg(feature = "usb-cdc")]
pub mod usb;

pub use architecture::{
    ArchitectureBackend, ArchitectureOperations, CONTEXT_RECORD_WORDS, ContextRecord,
    FaultRegister, SavedContextLayout,
};
pub use board::{
    BoardBackend, BoardError, BoardInfo, BoardServices, MemoryProtectionOperations, ResetCause,
    WatchdogBackend,
};
#[cfg(feature = "usb-cdc")]
pub use usb::{UsbInstallationReceive, UsbOperations, UsbResetDelay, UsbResources};
