//! Shared bounds for storage lifecycle recovery.

/// Maximum number of initialization attempts in one recovery window.
pub const REINITIALIZATION_ATTEMPTS: u32 = 3;

/// Delay between bounded reinitialization attempts.
pub const REINITIALIZATION_DELAY_MS: u32 = 100;

/// Delay between runtime card-presence probes in the recovery loop.
pub const RECOVERY_POLL_PERIOD_MS: u32 = 1_000;

const _: () = assert!(REINITIALIZATION_ATTEMPTS > 0);

#[cfg(test)]
mod tests {
    use super::RECOVERY_POLL_PERIOD_MS;

    #[test]
    fn recovery_probe_is_due_only_at_the_configured_period() {
        let recovery_poll_due = |elapsed_ms| elapsed_ms >= RECOVERY_POLL_PERIOD_MS;
        assert!(!recovery_poll_due(RECOVERY_POLL_PERIOD_MS - 1));
        assert!(recovery_poll_due(RECOVERY_POLL_PERIOD_MS));
    }
}
