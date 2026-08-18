//! Explicit recovery policy for the boot-only application runtime.

/// Action taken after the kernel terminates the active application.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FaultRecoveryAction {
    /// Keep the kernel alive in its safe recovery loop until an external reset.
    EnterKernelHeartbeat,
}

/// Restart behavior after an application fault.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RestartPolicy {
    /// A terminated context cannot be resumed or restarted automatically.
    ManualResetOnly,
}

/// Package rollback behavior for the read-only storage boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RollbackPolicy {
    /// No previous package can be selected or written by the current loader.
    UnavailableOnReadOnlyStorage,
}

/// Watchdog behavior before a bounded feed owner exists.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WatchdogPolicy {
    /// Do not arm hardware watchdogs without an explicit feed contract.
    DisabledUntilHeartbeatContract,
}

/// Kernel-owned decisions for the current single-context boot contract.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LifecyclePolicy {
    fault_recovery: FaultRecoveryAction,
    restart: RestartPolicy,
    rollback: RollbackPolicy,
    watchdog: WatchdogPolicy,
}

impl LifecyclePolicy {
    /// Returns the policy used by the current read-only boot path.
    pub const fn current() -> Self {
        Self {
            fault_recovery: FaultRecoveryAction::EnterKernelHeartbeat,
            restart: RestartPolicy::ManualResetOnly,
            rollback: RollbackPolicy::UnavailableOnReadOnlyStorage,
            watchdog: WatchdogPolicy::DisabledUntilHeartbeatContract,
        }
    }

    /// Returns the post-fault action.
    pub const fn fault_recovery(self) -> FaultRecoveryAction {
        self.fault_recovery
    }

    /// Returns the restart rule.
    pub const fn restart(self) -> RestartPolicy {
        self.restart
    }

    /// Returns the rollback rule.
    pub const fn rollback(self) -> RollbackPolicy {
        self.rollback
    }

    /// Returns the watchdog rule.
    pub const fn watchdog(self) -> WatchdogPolicy {
        self.watchdog
    }
}

/// Single source of truth for the active application lifecycle policy.
pub const CURRENT: LifecyclePolicy = LifecyclePolicy::current();

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_faulted_application_in_kernel_recovery() {
        assert_eq!(
            CURRENT.fault_recovery(),
            FaultRecoveryAction::EnterKernelHeartbeat
        );
        assert_eq!(CURRENT.restart(), RestartPolicy::ManualResetOnly);
    }

    #[test]
    fn rejects_rollback_without_writable_package_storage() {
        assert_eq!(
            CURRENT.rollback(),
            RollbackPolicy::UnavailableOnReadOnlyStorage
        );
    }

    #[test]
    fn does_not_arm_a_watchdog_without_a_feed_owner() {
        assert_eq!(
            CURRENT.watchdog(),
            WatchdogPolicy::DisabledUntilHeartbeatContract
        );
    }
}
