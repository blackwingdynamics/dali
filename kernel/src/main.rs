#![no_std]
#![no_main]
#![warn(clippy::missing_docs_in_private_items)]

pub(crate) mod abi;
mod bootstrap;
pub(crate) mod drivers;
pub mod loader;
#[cfg(all(feature = "abi-relocation", not(feature = "repository-loader")))]
#[path = "loader/contract/mod.rs"]
pub(crate) mod loader_contract;
pub mod logging;
pub(crate) mod platform;
pub mod runtime;
pub mod storage;

mod security;

use cortex_m_rt::entry;
use panic_halt as _;

/// Starts the kernel entry sequence.
#[entry]
fn main() -> ! {
    bootstrap::run()
}
