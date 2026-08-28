//! Kernel heartbeat loop.

#[cfg(feature = "sdio")]
use super::status::FAST_BLINK_PERIOD_MS;
#[cfg(feature = "sdio")]
/// Defines the `FAILURE_LOG_PERIOD_MS` bound used by this subsystem.
/// Defines the `FAILURE_LOG_PERIOD_MS` bound used by this subsystem.
const FAILURE_LOG_PERIOD_MS: u32 = 2_000;
use super::status::{
    HEARTBEAT_PERIOD_MS, SAFE_MODE_BLINK_PERIOD_MS, SAFE_MODE_LOG_PERIOD_MS, SLOW_BLINK_PERIOD_MS,
    StorageStatus,
};
use crate::{bootstrap::StorageRuntime, platform};

/// Displays the storage status and polls retained storage during recovery.
pub fn run<B>(mut board: platform::Platform<B>, mut storage: StorageRuntime<B>) -> !
where
    B: dali_kernel_api::BoardBackend,
    B::Watchdog: dali_kernel_api::WatchdogBackend,
{
    let mut led_on = false;
    let mut elapsed_ms = 0;
    let safe_mode = matches!(storage.status(), StorageStatus::SafeMode);
    let mut safe_mode_log_elapsed_ms = 0;
    #[cfg(feature = "sdio")]
    let mut failure_log_elapsed_ms = 0;
    #[cfg(feature = "sdio")]
    if matches!(storage.status(), StorageStatus::Removed) {
        crate::logging::info(
            crate::logging::BOOT_SUBSYSTEM,
            format_args!("[STORAGE] Recovery heartbeat active"),
        );
    }
    if safe_mode {
        crate::logging::info(
            crate::logging::SECURITY_SUBSYSTEM,
            format_args!("[RECOVERY] Safe Mode heartbeat active"),
        );
    }

    loop {
        board.poll_user_key();
        #[cfg(feature = "sdio")]
        if matches!(storage.status(), StorageStatus::Removed)
            && storage.recovery_poll_due(elapsed_ms)
        {
            storage.poll_recovery(&mut board);
            elapsed_ms = 0;
        }
        match storage.status() {
            #[cfg(feature = "sdio")]
            StorageStatus::Ready => {
                let _ = board.set_status_led(true);
            }
            StorageStatus::Idle => {
                if elapsed_ms >= SLOW_BLINK_PERIOD_MS {
                    led_on = !led_on;
                    let _ = board.set_status_led(led_on);
                    elapsed_ms = 0;
                }
            }
            #[cfg(not(feature = "sdio"))]
            StorageStatus::NotDetected => {
                if elapsed_ms >= SLOW_BLINK_PERIOD_MS {
                    led_on = !led_on;
                    let _ = board.set_status_led(led_on);
                    elapsed_ms = 0;
                }
            }
            #[cfg(feature = "sdio")]
            StorageStatus::Removed => {
                if elapsed_ms >= SLOW_BLINK_PERIOD_MS {
                    led_on = !led_on;
                    let _ = board.set_status_led(led_on);
                    elapsed_ms = 0;
                }
            }
            StorageStatus::SafeMode => {
                if elapsed_ms >= SAFE_MODE_BLINK_PERIOD_MS {
                    led_on = !led_on;
                    let _ = board.set_status_led(led_on);
                    elapsed_ms = 0;
                }
            }
            #[cfg(feature = "sdio")]
            StorageStatus::Failure => {
                if elapsed_ms >= FAST_BLINK_PERIOD_MS {
                    led_on = !led_on;
                    let _ = board.set_status_led(led_on);
                    elapsed_ms = 0;
                }
                if failure_log_elapsed_ms >= FAILURE_LOG_PERIOD_MS {
                    crate::logging::warn(
                        crate::logging::BOOT_SUBSYSTEM,
                        format_args!("[STORAGE] Storage recovery fault heartbeat active"),
                    );
                    failure_log_elapsed_ms = 0;
                }
            }
        }
        let watchdog_refreshed = match board.service_watchdog() {
            Ok(()) => true,
            Err(error) => {
                crate::logging::error(
                    crate::logging::BOOT_SUBSYSTEM,
                    format_args!(
                        "[WATCHDOG] Feed failed: {:?}; allowing hardware reset",
                        error
                    ),
                );
                false
            }
        };
        if safe_mode && watchdog_refreshed && safe_mode_log_elapsed_ms >= SAFE_MODE_LOG_PERIOD_MS {
            crate::logging::info(
                crate::logging::SECURITY_SUBSYSTEM,
                format_args!("[RECOVERY] Safe Mode heartbeat active (IWDG refreshed)"),
            );
            safe_mode_log_elapsed_ms = 0;
        }
        let _ = board.delay_ms(HEARTBEAT_PERIOD_MS);
        elapsed_ms = elapsed_ms.saturating_add(HEARTBEAT_PERIOD_MS);
        safe_mode_log_elapsed_ms = safe_mode_log_elapsed_ms.saturating_add(HEARTBEAT_PERIOD_MS);
        #[cfg(feature = "sdio")]
        if matches!(storage.status(), StorageStatus::Failure) {
            failure_log_elapsed_ms = failure_log_elapsed_ms.saturating_add(HEARTBEAT_PERIOD_MS);
        }
    }
}
