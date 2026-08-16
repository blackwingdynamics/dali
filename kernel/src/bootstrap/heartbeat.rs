//! Kernel heartbeat loop.

#[cfg(feature = "sdio")]
use super::status::FAST_BLINK_PERIOD_MS;
use super::status::{HEARTBEAT_PERIOD_MS, SLOW_BLINK_PERIOD_MS, StorageStatus};
use crate::platform;
use cortex_m::prelude::_embedded_hal_blocking_delay_DelayMs;

/// Displays the storage status through the board's single status LED forever.
pub fn run(mut board: platform::Board, storage_status: StorageStatus) -> ! {
    let mut led_on = false;
    let mut elapsed_ms = 0;

    loop {
        match storage_status {
            #[cfg(all(feature = "sdio", not(feature = "abi-v3-mpu")))]
            StorageStatus::Ready => {
                platform::set_status_led(&mut board, true);
            }
            StorageStatus::NotDetected => {
                if elapsed_ms >= SLOW_BLINK_PERIOD_MS {
                    led_on = !led_on;
                    platform::set_status_led(&mut board, led_on);
                    elapsed_ms = 0;
                }
            }
            #[cfg(feature = "sdio")]
            StorageStatus::Failure => {
                if elapsed_ms >= FAST_BLINK_PERIOD_MS {
                    led_on = !led_on;
                    platform::set_status_led(&mut board, led_on);
                    elapsed_ms = 0;
                }
            }
        }
        board.delay.delay_ms(HEARTBEAT_PERIOD_MS);
        elapsed_ms = elapsed_ms.saturating_add(HEARTBEAT_PERIOD_MS);
    }
}
