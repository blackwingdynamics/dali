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

/// Watchdog arming behavior for the current kernel heartbeat owner.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WatchdogPolicy {
    /// Arm and feed the watchdog only from the kernel heartbeat.
    KernelHeartbeat,
}

/// Action after the kernel loses the watchdog feed path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WatchdogFailureAction {
    /// Stop issuing feeds and let the armed hardware watchdog reset the target.
    AllowHardwareReset,
}

/// Kernel-owned decisions for the current single-context boot contract.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LifecyclePolicy {
    fault_recovery: FaultRecoveryAction,
    restart: RestartPolicy,
    rollback: RollbackPolicy,
    watchdog: WatchdogPolicy,
    watchdog_failure: WatchdogFailureAction,
}

impl LifecyclePolicy {
    /// Returns the policy used by the current read-only boot path.
    pub const fn current() -> Self {
        Self {
            fault_recovery: FaultRecoveryAction::EnterKernelHeartbeat,
            restart: RestartPolicy::ManualResetOnly,
            rollback: RollbackPolicy::UnavailableOnReadOnlyStorage,
            watchdog: WatchdogPolicy::KernelHeartbeat,
            watchdog_failure: WatchdogFailureAction::AllowHardwareReset,
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

    /// Returns the action used after a watchdog feed failure.
    pub const fn watchdog_failure(self) -> WatchdogFailureAction {
        self.watchdog_failure
    }
}

/// Single source of truth for the active application lifecycle policy.
pub const CURRENT: LifecyclePolicy = LifecyclePolicy::current();

#[cfg(test)]
mod tests {
    #[test]
    fn keeps_faulted_application_in_kernel_recovery() {
        assert_eq!(
            super::CURRENT.fault_recovery(),
            super::FaultRecoveryAction::EnterKernelHeartbeat
        );
        assert_eq!(
            super::CURRENT.restart(),
            super::RestartPolicy::ManualResetOnly
        );
    }

    #[test]
    fn rejects_rollback_without_writable_package_storage() {
        assert_eq!(
            super::CURRENT.rollback(),
            super::RollbackPolicy::UnavailableOnReadOnlyStorage
        );
    }

    #[test]
    fn arms_the_watchdog_only_from_the_kernel_heartbeat() {
        assert_eq!(
            super::CURRENT.watchdog(),
            super::WatchdogPolicy::KernelHeartbeat
        );
        assert_eq!(
            super::CURRENT.watchdog_failure(),
            super::WatchdogFailureAction::AllowHardwareReset
        );
    }
}
