//! Storage discovery, filesystem acceptance, and cartridge handoff.

#[cfg(feature = "storage-write")]
mod acceptance;
mod initialization;
#[cfg(feature = "storage-interruption-test")]
pub(super) mod interruption;
#[cfg(feature = "sdio")]
pub(super) mod recovery;

pub(super) use initialization::initialize;

use super::StorageRuntime;
#[cfg(feature = "sdio")]
use crate::platform;

#[cfg(feature = "sdio")]
/// Performs the `poll_runtime` operation for this subsystem.
pub(super) fn poll_runtime<B>(runtime: &mut StorageRuntime<B>, board: &mut platform::Platform<B>)
where
    B: dali_kernel_api::BoardBackend,
    B::Watchdog: dali_kernel_api::WatchdogBackend,
{
    recovery::poll_runtime(runtime, board);
}
