//! Shared bounds for storage lifecycle recovery.

/// Maximum number of initialization attempts in one recovery window.
pub const REINITIALIZATION_ATTEMPTS: u32 = 3;

/// Delay between bounded reinitialization attempts.
pub const REINITIALIZATION_DELAY_MS: u32 = 100;

const _: () = assert!(REINITIALIZATION_ATTEMPTS > 0);
