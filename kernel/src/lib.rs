#![cfg_attr(not(test), no_std)]
#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

//! Hardware-independent kernel contracts exposed for host-side testing.

mod bootstrap;
pub mod drivers;
pub mod loader;
#[cfg(not(feature = "repository-loader"))]
#[path = "loader/contract/mod.rs"]
pub mod loader_contract;
#[cfg(feature = "repository-loader")]
#[path = "loader/repository/mod.rs"]
pub mod repository_loader;
#[cfg(feature = "repository-loader")]
pub use repository_loader::streaming as repository_streaming;
pub mod logging;
pub(crate) mod platform;
pub mod runtime;
mod security;
pub mod storage;

/// Starts the kernel bootstrap with an externally selected board backend.
pub fn run<B>() -> !
where
    B: dali_kernel_api::BoardBackend,
    B::Watchdog: dali_kernel_api::WatchdogBackend,
{
    bootstrap::run::<B>()
}
