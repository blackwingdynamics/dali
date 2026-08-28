#![no_std]
#![no_main]

use cortex_m_rt::entry;
use dali_board_stm32f405::Board;
use panic_halt as _;

/// Starts the universal kernel with the selected backend composition.
#[entry]
fn main() -> ! {
    dali_kernel_core::run::<Board>()
}
