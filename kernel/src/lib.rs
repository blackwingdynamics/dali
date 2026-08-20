#![cfg_attr(not(test), no_std)]

//! Hardware-independent kernel contracts exposed for host-side testing.

pub mod drivers;
#[path = "loader/contract/mod.rs"]
pub mod loader_contract;
#[path = "security/mpu/mod.rs"]
pub mod mpu;
#[cfg(feature = "repository-loader")]
#[path = "loader/repository.rs"]
pub mod repository_loader;
#[cfg(feature = "repository-loader")]
#[path = "loader/repository_streaming.rs"]
pub mod repository_streaming;
pub mod runtime;
pub mod storage;
