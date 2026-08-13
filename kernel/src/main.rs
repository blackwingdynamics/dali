#![no_std]
#![no_main]

mod board;
pub mod logging;

use cortex_m::prelude::_embedded_hal_blocking_delay_DelayMs;
use cortex_m_rt::entry;
use panic_halt as _;
use stm32f4xx_hal::pac;

#[entry]
fn main() -> ! {
    logging::initialize();
    logging::info(
        logging::BOOT_SUBSYSTEM,
        format_args!("===================================="),
    );
    logging::info(
        logging::BOOT_SUBSYSTEM,
        format_args!("   Dali OS Kernel Booting...       "),
    );
    logging::info(
        logging::BOOT_SUBSYSTEM,
        format_args!("===================================="),
    );

    // The reset entry point runs once, so both peripheral singleton tokens are available.
    let device = pac::Peripherals::take().unwrap();
    let core = cortex_m::Peripherals::take().unwrap();
    let mut board: board::Board = board::initialize(device, core);

    #[cfg(feature = "board-stm32f405-sd")]
    let _sdio_pins = board.take_sdio_pins();

    logging::info(
        logging::BOOT_SUBSYSTEM,
        format_args!("[BOOT] System clock: {} MHz", board::SYSTEM_CLOCK_MHZ),
    );

    logging::info(
        logging::BOOT_SUBSYSTEM,
        format_args!("Hardware bootstrap complete"),
    );
    logging::info(
        logging::BOOT_SUBSYSTEM,
        format_args!("Entering kernel heartbeat"),
    );

    let mut heartbeat_counter: u64 = 0;

    loop {
        board.status_led.toggle();
        heartbeat_counter = heartbeat_counter.wrapping_add(1);

        logging::info(
            logging::BOOT_SUBSYSTEM,
            format_args!("Heartbeat: {}", heartbeat_counter),
        );

        board.delay.delay_ms(board::HEARTBEAT_PERIOD_MS);
    }
}
