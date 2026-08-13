//! Kernel startup orchestration.

mod heartbeat;

use crate::{board, logging};
use stm32f4xx_hal::pac;

/// Runs the kernel bootstrap sequence and enters the heartbeat loop.
pub fn run() -> ! {
    initialize_logging();
    emit_boot_banner();

    // The reset entry point runs once, so both peripheral singleton tokens are available.
    let device = pac::Peripherals::take().unwrap();
    let core = cortex_m::Peripherals::take().unwrap();
    #[cfg(feature = "board-stm32f405-sd")]
    let mut board = board::initialize(device, core);

    #[cfg(not(feature = "board-stm32f405-sd"))]
    let board = board::initialize(device, core);

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

    heartbeat::run(board);
}

fn initialize_logging() {
    logging::initialize();
}

fn emit_boot_banner() {
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
}
