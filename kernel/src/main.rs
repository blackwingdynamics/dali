#![no_std]
#![no_main]

pub(crate) mod board;
mod bootstrap;
pub mod drivers;
pub mod logging;
pub mod storage;

use cortex_m_rt::entry;
use panic_halt as _;

#[entry]
fn main() -> ! {
    bootstrap::run()
}
