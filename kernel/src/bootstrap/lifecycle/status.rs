//! Bootstrap status states and timing constants.

use crate::runtime::watchdog::ResetCause;

/// Boot policy selected from the reset cause before storage or applications
/// are initialized.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BootMode {
    /// Normal boot may continue to storage and application loading.
    Normal,
    /// Recovery boot keeps applications from starting automatically.
    SafeMode,
}

impl BootMode {
    /// Selects recovery boot after a hardware watchdog reset.
    pub const fn from_reset_cause(cause: ResetCause) -> Self {
        match cause {
            ResetCause::Watchdog => Self::SafeMode,
            ResetCause::PowerOn
            | ResetCause::External
            | ResetCause::Software
            | ResetCause::Unknown => Self::Normal,
        }
    }
}

/// Result of the storage bring-up sequence.
#[derive(Clone, Copy)]
pub enum StorageStatus {
    /// No storage medium was detected or the transport is not configured.
    #[cfg(not(feature = "sdio"))]
    NotDetected,
    /// A previously probed card stopped responding and recovery is active.
    #[cfg(feature = "sdio")]
    Removed,
    /// The storage medium is available, but no application package is present.
    Idle,
    /// The card initialized and block zero was read successfully.
    #[cfg(feature = "sdio")]
    Ready,
    /// The card or transport reported an operational failure.
    #[cfg(feature = "sdio")]
    Failure,
    /// Recovery boot intentionally skipped package loading.
    SafeMode,
}

/// Delay between slow status LED transitions.
pub const SLOW_BLINK_PERIOD_MS: u32 = 1_000;

/// Delay between fast status LED transitions.
#[cfg(feature = "sdio")]
pub const FAST_BLINK_PERIOD_MS: u32 = 100;

/// Delay between safe-mode status LED transitions.
pub const SAFE_MODE_BLINK_PERIOD_MS: u32 = 250;

/// Delay between Safe Mode watchdog refresh diagnostics.
pub const SAFE_MODE_LOG_PERIOD_MS: u32 = 1_000;

/// Heartbeat loop tick used for bounded status LED timing.
pub const HEARTBEAT_PERIOD_MS: u32 = 10;

#[cfg(test)]
mod tests {
    use super::{BootMode, ResetCause};

    #[test]
    fn watchdog_reset_enters_safe_mode() {
        assert_eq!(
            BootMode::from_reset_cause(ResetCause::Watchdog),
            BootMode::SafeMode
        );
    }

    #[test]
    fn non_watchdog_resets_keep_normal_boot() {
        for cause in [
            ResetCause::PowerOn,
            ResetCause::External,
            ResetCause::Software,
            ResetCause::Unknown,
        ] {
            assert_eq!(BootMode::from_reset_cause(cause), BootMode::Normal);
        }
    }
}
