//! Kernel heartbeat loop.

#[cfg(feature = "sdio")]
use super::status::FAST_BLINK_PERIOD_MS;
use super::status::{HEARTBEAT_PERIOD_MS, SLOW_BLINK_PERIOD_MS, StorageStatus};
use crate::platform;
use crate::runtime::application::policy::{CURRENT, WatchdogFailureAction};
use crate::runtime::watchdog::FeedOwner;

/// Displays the storage status through the board's single status LED forever.
pub fn run(
    mut board: platform::Platform,
    storage_status: StorageStatus,
    mut watchdog: Option<platform::WatchdogRuntime>,
) -> ! {
    let mut led_on = false;
    let mut elapsed_ms = 0;

    if let Some(runtime) = watchdog.as_mut()
        && let Err(error) = runtime.arm(FeedOwner::KernelHeartbeat)
    {
        crate::logging::error(
            crate::logging::BOOT_SUBSYSTEM,
            format_args!("[WATCHDOG] Failed to arm: {:?}", error),
        );
        watchdog = None;
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
        if let Some(runtime) = watchdog.as_mut()
            && let Err(error) = runtime.feed(FeedOwner::KernelHeartbeat)
        {
            crate::logging::error(
                crate::logging::BOOT_SUBSYSTEM,
                format_args!(
                    "[WATCHDOG] Feed failed: {:?}; allowing hardware reset",
                    error
                ),
            );
            if CURRENT.watchdog_failure() == WatchdogFailureAction::AllowHardwareReset {
                watchdog = None;
            }
        }
    }
}
