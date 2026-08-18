#![no_std]
#![no_main]

pub(crate) mod abi;
mod bootstrap;
pub(crate) mod drivers;
pub mod loader;
#[cfg(feature = "abi-relocation")]
#[path = "loader/contract.rs"]
pub(crate) mod loader_contract;
pub mod logging;
pub(crate) mod platform;
pub mod runtime;
pub mod storage;

mod security;

use cortex_m_rt::entry;
use panic_halt as _;

#[entry]
fn main() -> ! {
    bootstrap::run()
}
