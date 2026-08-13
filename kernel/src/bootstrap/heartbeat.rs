//! Kernel heartbeat loop.

#[cfg(feature = "board-stm32f405-sd")]
use super::status::{FAST_BLINK_PERIOD_MS, READY_STATUS_PERIOD_MS};
use super::status::{SLOW_BLINK_PERIOD_MS, StorageStatus};
use crate::board;
use cortex_m::prelude::_embedded_hal_blocking_delay_DelayMs;

/// Displays the storage status through the board's single status LED forever.
pub fn run(mut board: board::Board, storage_status: StorageStatus) -> ! {
    let mut led_on = false;

    loop {
        match storage_status {
            #[cfg(feature = "board-stm32f405-sd")]
            StorageStatus::Ready => {
                board::set_status_led(&mut board, true);
                board.delay.delay_ms(READY_STATUS_PERIOD_MS);
            }
            StorageStatus::NotDetected => {
                led_on = !led_on;
                board::set_status_led(&mut board, led_on);
                board.delay.delay_ms(SLOW_BLINK_PERIOD_MS);
            }
            #[cfg(feature = "board-stm32f405-sd")]
            StorageStatus::Failure => {
                led_on = !led_on;
                board::set_status_led(&mut board, led_on);
                board.delay.delay_ms(FAST_BLINK_PERIOD_MS);
            }
        }
    }
}
