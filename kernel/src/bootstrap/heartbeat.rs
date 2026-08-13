//! Kernel heartbeat loop.

#[cfg(feature = "board-stm32f405-sd")]
use super::status::FAST_BLINK_PERIOD_MS;
use super::status::{SLOW_BLINK_PERIOD_MS, StorageStatus, USB_SERVICE_PERIOD_MS};
use crate::board;
use cortex_m::prelude::_embedded_hal_blocking_delay_DelayMs;

/// Displays the storage status through the board's single status LED forever.
pub fn run(mut board: board::Board, storage_status: StorageStatus) -> ! {
    let mut led_on = false;
    let mut elapsed_ms = 0;

    loop {
        crate::logging::poll();
        match storage_status {
            #[cfg(feature = "board-stm32f405-sd")]
            StorageStatus::Ready => {
                board::set_status_led(&mut board, true);
            }
            StorageStatus::NotDetected => {
                if elapsed_ms >= SLOW_BLINK_PERIOD_MS {
                    led_on = !led_on;
                    board::set_status_led(&mut board, led_on);
                    elapsed_ms = 0;
                }
            }
            #[cfg(feature = "board-stm32f405-sd")]
            StorageStatus::Failure => {
                if elapsed_ms >= FAST_BLINK_PERIOD_MS {
                    led_on = !led_on;
                    board::set_status_led(&mut board, led_on);
                    elapsed_ms = 0;
                }
            }
        }
        board.delay.delay_ms(USB_SERVICE_PERIOD_MS);
        elapsed_ms = elapsed_ms.saturating_add(USB_SERVICE_PERIOD_MS);
    }
}
