#![cfg_attr(not(test), no_std)]

//! Hardware-independent kernel contracts exposed for host-side testing.

pub mod drivers;
#[path = "loader/contract/mod.rs"]
pub mod loader_contract;
#[path = "security/mpu/mod.rs"]
pub mod mpu;
#[cfg(feature = "repository-loader")]
#[path = "loader/repository/mod.rs"]
pub mod repository_loader;
#[cfg(feature = "repository-loader")]
pub use repository_loader::streaming as repository_streaming;
pub mod runtime;
pub mod storage;
