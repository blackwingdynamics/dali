//! Storage discovery, filesystem acceptance, and package handoff.

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
pub(super) fn poll_runtime(runtime: &mut StorageRuntime, board: &mut platform::Platform) {
    recovery::poll_runtime(runtime, board);
}
