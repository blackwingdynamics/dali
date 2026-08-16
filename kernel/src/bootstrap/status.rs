//! Bootstrap status states and timing constants.

/// Result of the storage bring-up sequence.
#[derive(Clone, Copy)]
pub enum StorageStatus {
    /// No storage medium was detected or the transport is not configured.
    NotDetected,
    /// The card initialized and block zero was read successfully.
    #[cfg(all(feature = "sdio", not(feature = "abi-v3-mpu")))]
    Ready,
    /// The card or transport reported an operational failure.
    #[cfg(feature = "sdio")]
    Failure,
}

/// Delay between slow status LED transitions.
pub const SLOW_BLINK_PERIOD_MS: u32 = 1_000;

/// Delay between fast status LED transitions.
#[cfg(feature = "sdio")]
pub const FAST_BLINK_PERIOD_MS: u32 = 100;

/// Heartbeat loop tick used for bounded status LED timing.
pub const HEARTBEAT_PERIOD_MS: u32 = 10;
