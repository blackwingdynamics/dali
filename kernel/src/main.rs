#![no_std]
#![no_main]

pub(crate) mod abi;
pub(crate) mod board;
mod bootstrap;
pub mod loader;
pub mod logging;
pub(crate) mod platform;
pub mod storage;

#[cfg(feature = "abi-current")]
mod security;

use cortex_m_rt::entry;
use panic_halt as _;

#[entry]
fn main() -> ! {
    bootstrap::run()
}
