//! Bootstrap status states and timing constants.

/// Result of the storage bring-up sequence.
#[derive(Clone, Copy)]
pub enum StorageStatus {
    /// No storage medium was detected or the transport is not configured.
    NotDetected,
    /// The card initialized and block zero was read successfully.
    #[cfg(feature = "board-stm32f405-sd")]
    Ready,
    /// The card or transport reported an operational failure.
    #[cfg(feature = "board-stm32f405-sd")]
    Failure,
}

/// Delay between slow status LED transitions.
pub const SLOW_BLINK_PERIOD_MS: u32 = 1_000;

/// Delay between fast status LED transitions.
#[cfg(feature = "board-stm32f405-sd")]
pub const FAST_BLINK_PERIOD_MS: u32 = 100;

/// Maximum interval between USB CDC service calls.
pub const USB_SERVICE_PERIOD_MS: u32 = 10;

/// Bounded host-enumeration and CDC control-handshake deadline.
pub const USB_ENUMERATION_TIMEOUT_MS: u32 = 5_000;
