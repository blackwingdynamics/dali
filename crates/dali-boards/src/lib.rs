#![no_std]
#![warn(missing_docs)]

//! Compatibility facade for the public Dali kernel integration API.
//!
//! New backend crates may depend directly on `dali-kernel-api`. This facade is
//! retained for source compatibility with existing backend implementations.

pub use dali_kernel_api::{
    BoardBackend, BoardError, BoardInfo, BoardServices, MemoryProtectionProvider, ResetCause,
    WatchdogBackend,
};
pub use dali_kernel_api::{board, dma, storage};
