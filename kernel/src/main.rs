#![no_std]
#![no_main]

mod board;

use cortex_m::prelude::_embedded_hal_blocking_delay_DelayMs;
use cortex_m_rt::entry;
use panic_halt as _;
use rtt_target::{rprintln, rtt_init_print};
use stm32f4xx_hal::pac;

#[entry]
fn main() -> ! {
    // Initialize Segger RTT channel for host logging.
    rtt_init_print!();
    rprintln!("====================================");
    rprintln!("   Dali OS Kernel Booting...       ");
    rprintln!("====================================");

    // The reset entry point runs once, so both peripheral singleton tokens are available.
    let device = pac::Peripherals::take().unwrap();
    let core = cortex_m::Peripherals::take().unwrap();
    let mut board: board::Board = board::initialize(device, core);

    rprintln!("[BOOT] System clock: {} MHz", board::SYSTEM_CLOCK_MHZ);

    rprintln!("[BOOT] Hardware bootstrap complete");
    rprintln!("[BOOT] Entering kernel heartbeat");

    let mut heartbeat_counter: u64 = 0;

    loop {
        board.status_led.toggle();
        heartbeat_counter = heartbeat_counter.wrapping_add(1);

        rprintln!("[BOOT] Heartbeat: {}", heartbeat_counter);

        board.delay.delay_ms(board::HEARTBEAT_PERIOD_MS);
    }
}
