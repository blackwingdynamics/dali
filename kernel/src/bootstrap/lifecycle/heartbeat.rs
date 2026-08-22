//! Kernel heartbeat loop.

#[cfg(feature = "sdio")]
use super::status::FAST_BLINK_PERIOD_MS;
use super::status::{
    HEARTBEAT_PERIOD_MS, SAFE_MODE_BLINK_PERIOD_MS, SLOW_BLINK_PERIOD_MS, StorageStatus,
};
use crate::platform;

/// Displays the storage status through the board's single status LED forever.
pub fn run(mut board: platform::Platform, storage_status: StorageStatus) -> ! {
    let mut led_on = false;
    let mut elapsed_ms = 0;
    #[cfg(feature = "sdio")]
    if matches!(storage_status, StorageStatus::Removed) {
        crate::logging::info(
            crate::logging::BOOT_SUBSYSTEM,
            format_args!("[STORAGE] Recovery heartbeat active"),
        );
    }

    loop {
        match storage_status {
            #[cfg(all(feature = "sdio", not(feature = "abi-mpu")))]
            StorageStatus::Ready => {
                board.set_status_led(true);
            }
            StorageStatus::Idle => {
                if elapsed_ms >= SLOW_BLINK_PERIOD_MS {
                    led_on = !led_on;
                    board.set_status_led(led_on);
                    elapsed_ms = 0;
                }
            }
            StorageStatus::NotDetected => {
                if elapsed_ms >= SLOW_BLINK_PERIOD_MS {
                    led_on = !led_on;
                    board.set_status_led(led_on);
                    elapsed_ms = 0;
                }
            }
            #[cfg(feature = "sdio")]
            StorageStatus::Removed => {
                if elapsed_ms >= SLOW_BLINK_PERIOD_MS {
                    led_on = !led_on;
                    board.set_status_led(led_on);
                    elapsed_ms = 0;
                }
            }
            StorageStatus::SafeMode => {
                if elapsed_ms >= SAFE_MODE_BLINK_PERIOD_MS {
                    led_on = !led_on;
                    board.set_status_led(led_on);
                    elapsed_ms = 0;
                }
            }
            #[cfg(feature = "sdio")]
            StorageStatus::Failure => {
                if elapsed_ms >= FAST_BLINK_PERIOD_MS {
                    led_on = !led_on;
                    board.set_status_led(led_on);
                    elapsed_ms = 0;
                }
            }
        }
        board.delay_ms(HEARTBEAT_PERIOD_MS);
        elapsed_ms = elapsed_ms.saturating_add(HEARTBEAT_PERIOD_MS);
        if let Err(error) = platform::service_watchdog() {
            crate::logging::error(
                crate::logging::BOOT_SUBSYSTEM,
                format_args!(
                    "[WATCHDOG] Feed failed: {:?}; allowing hardware reset",
                    error
                ),
            );
        }
    }
}
