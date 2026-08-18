//! Bounded application lifecycle state owned by the kernel runtime.

use dali_amrn::v4::PACKAGE_ID_LENGTH;

use super::slots::{SlotAllocation, SlotManagerError};

/// Opaque identity assigned to one installed application package.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ApplicationIdentity([u8; PACKAGE_ID_LENGTH]);

impl ApplicationIdentity {
    /// Creates an identity from validated package metadata.
    pub const fn new(bytes: [u8; PACKAGE_ID_LENGTH]) -> Self {
        Self(bytes)
    }

    /// Returns the opaque package identity bytes.
    pub const fn bytes(self) -> [u8; PACKAGE_ID_LENGTH] {
        self.0
    }
}

/// States in the bounded single-context application lifecycle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApplicationState {
    /// Metadata was accepted, but the image has not been copied yet.
    Discovered,
    /// The image was copied into its reserved slot.
    Loaded,
    /// The launch frame and protection preparation are complete.
    Ready,
    /// The application owns the active unprivileged execution context.
    Running,
    /// The application raised a fault and execution was terminated.
    Faulted,
    /// Kernel recovery is processing the terminated application.
    Recovering,
    /// The application context cannot be resumed or restarted implicitly.
    Terminated,
}

/// Errors returned when an application lifecycle transition is invalid.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LifecycleError {
    /// The requested transition is not valid for the current state.
    InvalidTransition {
        state: ApplicationState,
        event: LifecycleEvent,
    },
    /// The reserved slot does not match the package's declared slot.
    SlotMismatch,
    /// The slot manager rejected the lifecycle-owned allocation.
    SlotManager(SlotManagerError),
}

/// Events accepted by the lifecycle transition function.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LifecycleEvent {
    /// The image finished copying into its reserved slot.
    Loaded,
    /// Launch preparation and protection setup completed.
    Ready,
    /// The application entered unprivileged execution.
    Started,
    /// The kernel fault boundary terminated application execution.
    Faulted,
    /// Kernel recovery began after the fault boundary captured the fault.
    RecoveryStarted,
    /// Kernel recovery completed and the context was retired.
    Terminated,
}

/// Kernel-owned lifecycle record for one application identity and slot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ApplicationLifecycle {
    identity: ApplicationIdentity,
    declared_slot: u8,
    allocation: Option<SlotAllocation>,
    state: ApplicationState,
}

impl ApplicationLifecycle {
    /// Creates a lifecycle record after package discovery and slot selection.
    pub const fn discovered(identity: ApplicationIdentity, declared_slot: u8) -> Self {
        Self {
            identity,
            declared_slot,
            allocation: None,
            state: ApplicationState::Discovered,
        }
    }

    /// Returns the package identity owned by this record.
    pub const fn identity(self) -> ApplicationIdentity {
        self.identity
    }

    /// Returns the package's manifest-declared slot identifier.
    pub const fn declared_slot(self) -> u8 {
        self.declared_slot
    }

    /// Returns the reserved slot after the image has been loaded.
    pub const fn allocation(self) -> Option<SlotAllocation> {
        self.allocation
    }

    /// Returns the current lifecycle state.
    pub const fn state(self) -> ApplicationState {
        self.state
    }

    /// Applies one bounded lifecycle event and returns the new state.
    pub fn transition(
        &mut self,
        event: LifecycleEvent,
    ) -> Result<ApplicationState, LifecycleError> {
        match (self.state, event) {
            (ApplicationState::Discovered, LifecycleEvent::Loaded) => {
                self.state = ApplicationState::Loaded;
            }
            (ApplicationState::Loaded, LifecycleEvent::Ready) => {
                self.state = ApplicationState::Ready;
            }
            (ApplicationState::Ready, LifecycleEvent::Started) => {
                self.state = ApplicationState::Running;
            }
            (ApplicationState::Running, LifecycleEvent::Faulted) => {
                self.state = ApplicationState::Faulted;
            }
            (ApplicationState::Faulted, LifecycleEvent::RecoveryStarted) => {
                self.state = ApplicationState::Recovering;
            }
            (ApplicationState::Recovering, LifecycleEvent::Terminated) => {
                self.state = ApplicationState::Terminated;
            }
            (state, event) => {
                return Err(LifecycleError::InvalidTransition { state, event });
            }
        }
        Ok(self.state)
    }

    /// Records the slot allocation after the image copy completes.
    pub fn record_allocation(
        &mut self,
        allocation: SlotAllocation,
    ) -> Result<ApplicationState, LifecycleError> {
        if self.state != ApplicationState::Discovered {
            return Err(LifecycleError::InvalidTransition {
                state: self.state,
                event: LifecycleEvent::Loaded,
            });
        }
        if allocation.slot().id != self.declared_slot {
            return Err(LifecycleError::SlotMismatch);
        }
        self.allocation = Some(allocation);
        self.transition(LifecycleEvent::Loaded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::slots::SlotManager;
    use dali_targets::IsolationSlot;

    const IDENTITY: ApplicationIdentity = ApplicationIdentity::new([1; PACKAGE_ID_LENGTH]);
    const SLOT: IsolationSlot = IsolationSlot {
        id: 0,
        name: "slot0",
        code_origin: 0x2000_8000,
        code_length: 0x4000,
        data_origin: 0x2000_C000,
        data_length: 0x4000,
        stack_length: 0x1000,
    };
    const OTHER_SLOT: IsolationSlot = IsolationSlot {
        id: 1,
        name: "slot1",
        code_origin: 0x2001_0000,
        code_length: 0x4000,
        data_origin: 0x2001_4000,
        data_length: 0x4000,
        stack_length: 0x1000,
    };

    static SLOTS: &[IsolationSlot] = &[SLOT, OTHER_SLOT];

    fn allocation(slot: IsolationSlot) -> SlotAllocation {
        let mut manager = SlotManager::new(SLOTS).expect("test slot table is valid");
        manager.reserve(slot).expect("test slot is declared")
    }

    #[test]
    fn accepts_the_complete_fault_recovery_lifecycle() {
        let mut lifecycle = ApplicationLifecycle::discovered(IDENTITY, SLOT.id);
        assert_eq!(
            lifecycle.record_allocation(allocation(SLOT)),
            Ok(ApplicationState::Loaded)
        );
        for event in [
            LifecycleEvent::Ready,
            LifecycleEvent::Started,
            LifecycleEvent::Faulted,
            LifecycleEvent::RecoveryStarted,
            LifecycleEvent::Terminated,
        ] {
            assert!(lifecycle.transition(event).is_ok());
        }
        assert_eq!(lifecycle.state(), ApplicationState::Terminated);
    }

    #[test]
    fn rejects_a_transition_that_skips_launch_preparation() {
        let mut lifecycle = ApplicationLifecycle::discovered(IDENTITY, SLOT.id);
        assert_eq!(
            lifecycle.transition(LifecycleEvent::Started),
            Err(LifecycleError::InvalidTransition {
                state: ApplicationState::Discovered,
                event: LifecycleEvent::Started,
            })
        );
    }

    #[test]
    fn rejects_an_allocation_for_a_different_declared_slot() {
        let mut lifecycle = ApplicationLifecycle::discovered(IDENTITY, SLOT.id);
        assert_eq!(
            lifecycle.record_allocation(allocation(OTHER_SLOT)),
            Err(LifecycleError::SlotMismatch)
        );
        assert_eq!(lifecycle.state(), ApplicationState::Discovered);
    }

    #[test]
    fn does_not_resume_a_terminated_context() {
        let mut lifecycle = ApplicationLifecycle::discovered(IDENTITY, SLOT.id);
        let _ = lifecycle.record_allocation(allocation(SLOT));
        for event in [
            LifecycleEvent::Ready,
            LifecycleEvent::Started,
            LifecycleEvent::Faulted,
            LifecycleEvent::RecoveryStarted,
            LifecycleEvent::Terminated,
        ] {
            let _ = lifecycle.transition(event);
        }
        assert!(matches!(
            lifecycle.transition(LifecycleEvent::Started),
            Err(LifecycleError::InvalidTransition {
                state: ApplicationState::Terminated,
                event: LifecycleEvent::Started,
            })
        ));
    }
}
