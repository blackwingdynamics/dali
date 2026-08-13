//! Kernel heartbeat loop.

use crate::{board, logging};
use cortex_m::prelude::_embedded_hal_blocking_delay_DelayMs;

/// Toggles the board status LED at the configured heartbeat interval forever.
pub fn run(mut board: board::Board) -> ! {
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
