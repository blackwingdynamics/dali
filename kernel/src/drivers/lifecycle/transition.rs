//! Deterministic storage lifecycle transition table.

use super::state::{StorageLifecycleEvent, StorageLifecycleState};

/// Bounded state holder for one storage transport instance.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StorageLifecycle {
    /// Stores the state associated with this bounded state.
    state: StorageLifecycleState,
}

impl StorageLifecycle {
    /// Creates a lifecycle that has not observed a storage medium.
    pub const fn new() -> Self {
        Self {
            state: StorageLifecycleState::Unavailable,
        }
    }

    /// Returns the current lifecycle state.
    pub const fn state(self) -> StorageLifecycleState {
        self.state
    }

    /// Applies one bounded hardware observation.
    pub fn apply(&mut self, event: StorageLifecycleEvent) -> StorageLifecycleState {
        self.state = match event {
            StorageLifecycleEvent::InitializationStarted => StorageLifecycleState::Present,
            StorageLifecycleEvent::InitializationSucceeded => StorageLifecycleState::Ready,
            StorageLifecycleEvent::CardRemoved => StorageLifecycleState::Removed,
            StorageLifecycleEvent::OperationFailed => StorageLifecycleState::Fault,
            StorageLifecycleEvent::OperationSucceeded => match self.state {
                StorageLifecycleState::Ready => StorageLifecycleState::Ready,
                state => state,
            },
        };
        self.state
    }
}

impl Default for StorageLifecycle {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn models_initialization_and_ready_operation() {
        let mut lifecycle = StorageLifecycle::new();
        assert_eq!(
            lifecycle.apply(StorageLifecycleEvent::InitializationStarted),
            StorageLifecycleState::Present
        );
        assert_eq!(
            lifecycle.apply(StorageLifecycleEvent::InitializationSucceeded),
            StorageLifecycleState::Ready
        );
        assert_eq!(
            lifecycle.apply(StorageLifecycleEvent::OperationSucceeded),
            StorageLifecycleState::Ready
        );
    }

    #[test]
    fn removal_enters_recovery_and_reinitialization_can_restore_ready() {
        let mut lifecycle = StorageLifecycle::new();
        lifecycle.apply(StorageLifecycleEvent::InitializationSucceeded);
        assert_eq!(
            lifecycle.apply(StorageLifecycleEvent::CardRemoved),
            StorageLifecycleState::Removed
        );
        lifecycle.apply(StorageLifecycleEvent::InitializationStarted);
        assert_eq!(
            lifecycle.apply(StorageLifecycleEvent::InitializationSucceeded),
            StorageLifecycleState::Ready
        );
    }

    #[test]
    fn non_removal_failure_is_fault_and_does_not_hide_the_fault() {
        let mut lifecycle = StorageLifecycle::new();
        lifecycle.apply(StorageLifecycleEvent::InitializationStarted);
        assert_eq!(
            lifecycle.apply(StorageLifecycleEvent::OperationFailed),
            StorageLifecycleState::Fault
        );
        assert_eq!(lifecycle.state(), StorageLifecycleState::Fault);
    }
}
