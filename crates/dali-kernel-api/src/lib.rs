#![no_std]
#![warn(missing_docs)]

//! Public, hardware-neutral contracts for integrating Dali OS backends.

pub mod architecture;
pub mod board;
pub mod dma;
pub mod storage;

pub use architecture::{ArchitectureBackend, ArchitectureOperations};
pub use board::{
    BoardBackend, BoardError, BoardInfo, BoardServices, MemoryProtectionOperations, ResetCause,
    WatchdogBackend,
};
