//! Ownership contract for the single active application context.

use core::sync::atomic::{AtomicU8, Ordering};

use super::lifecycle::{
    ApplicationIdentity, ApplicationLifecycle, ApplicationState, LifecycleError,
};
use crate::runtime::memory::slots::SlotAllocation;

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RuntimePhase {
    Inactive = 0,
    Running = 1,
    Faulted = 2,
    Recovering = 3,
    Terminated = 4,
}

/// Kernel-owned atomic state shared by launch and fault entry paths.
pub struct RuntimeState {
    phase: AtomicU8,
}

impl RuntimeState {
    /// Creates an inactive runtime state.
    pub const fn new() -> Self {
        Self {
            phase: AtomicU8::new(RuntimePhase::Inactive as u8),
        }
    }

    fn transition(&self, expected: RuntimePhase, next: RuntimePhase) -> bool {
        self.phase
            .compare_exchange(
                expected as u8,
                next as u8,
                Ordering::SeqCst,
                Ordering::SeqCst,
            )
            .is_ok()
    }

    fn publish_running(&self) {
        self.phase
            .store(RuntimePhase::Running as u8, Ordering::SeqCst);
    }

    /// Records a fault from the active application.
    pub(crate) fn record_fault(&self) -> bool {
        self.transition(RuntimePhase::Running, RuntimePhase::Faulted)
    }

    /// Records entry into kernel recovery.
    pub(crate) fn begin_recovery(&self) -> bool {
        self.transition(RuntimePhase::Faulted, RuntimePhase::Recovering)
    }

    /// Records terminal application recovery.
    pub(crate) fn terminate(&self) -> bool {
        self.transition(RuntimePhase::Recovering, RuntimePhase::Terminated)
    }

    fn is_terminated(&self) -> bool {
        self.phase.load(Ordering::SeqCst) == RuntimePhase::Terminated as u8
    }

    fn clear(&self) {
        self.phase
            .store(RuntimePhase::Inactive as u8, Ordering::SeqCst);
    }
}

impl Default for RuntimeState {
    fn default() -> Self {
        Self::new()
    }
}

/// Runtime state used by the active kernel context.
pub static ACTIVE_RUNTIME_STATE: RuntimeState = RuntimeState::new();

/// Records a fault from the kernel-owned exception boundary.
pub fn record_active_fault() -> bool {
    ACTIVE_RUNTIME_STATE.record_fault()
}

/// Records entry into the kernel-owned recovery context.
pub fn begin_active_recovery() -> bool {
    ACTIVE_RUNTIME_STATE.begin_recovery()
}

/// Records terminal application recovery.
pub fn terminate_active_context() -> bool {
    ACTIVE_RUNTIME_STATE.terminate()
}

/// The application context owned by the current runtime execution path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActiveContext {
    identity: ApplicationIdentity,
    allocation: SlotAllocation,
}

impl ActiveContext {
    /// Returns the identity of the running application.
    pub const fn identity(self) -> ApplicationIdentity {
        self.identity
    }

    /// Returns the manifest-owned slot held by the running application.
    pub const fn allocation(self) -> SlotAllocation {
        self.allocation
    }
}

/// Errors returned by the single-context ownership contract.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContextOwnerError {
    /// A second application attempted to become active.
    AlreadyActive,
    /// The lifecycle was not ready for activation.
    InvalidLifecycleState(ApplicationState),
    /// A ready lifecycle did not carry a reserved slot allocation.
    MissingAllocation,
    /// The lifecycle transition failed before activation completed.
    Lifecycle(LifecycleError),
    /// No context exists to retire.
    NoActiveContext,
    /// A context can be retired only after terminal recovery.
    NotTerminated,
    /// The atomic runtime phase did not match the lifecycle transition.
    RuntimeStateMismatch,
}

/// Kernel-owned holder for at most one active application context.
#[derive(Clone, Copy)]
pub struct ActiveContextOwner<'state> {
    active: Option<ApplicationLifecycle>,
    runtime_state: &'state RuntimeState,
}

impl<'state> ActiveContextOwner<'state> {
    /// Creates an owner with no active application context.
    pub const fn new(runtime_state: &'state RuntimeState) -> Self {
        Self {
            active: None,
            runtime_state,
        }
    }

    /// Returns the currently active context, if one exists.
    pub fn active(&self) -> Option<ActiveContext> {
        self.active.and_then(|lifecycle| {
            lifecycle.allocation().map(|allocation| ActiveContext {
                identity: lifecycle.identity(),
                allocation,
            })
        })
    }

    /// Activates one ready lifecycle and claims its slot for execution.
    pub fn activate(
        &mut self,
        lifecycle: &mut ApplicationLifecycle,
    ) -> Result<ActiveContext, ContextOwnerError> {
        if self.active.is_some() {
            return Err(ContextOwnerError::AlreadyActive);
        }
        if lifecycle.state() != ApplicationState::Ready {
            return Err(ContextOwnerError::InvalidLifecycleState(lifecycle.state()));
        }
        let allocation = lifecycle
            .allocation()
            .ok_or(ContextOwnerError::MissingAllocation)?;
        lifecycle
            .transition(super::lifecycle::LifecycleEvent::Started)
            .map_err(ContextOwnerError::Lifecycle)?;
        let context = ActiveContext {
            identity: lifecycle.identity(),
            allocation,
        };
        self.active = Some(*lifecycle);
        self.runtime_state.publish_running();
        Ok(context)
    }

    /// Applies a fault or recovery transition to the owned lifecycle.
    pub fn transition(
        &mut self,
        event: super::lifecycle::LifecycleEvent,
    ) -> Result<ApplicationState, ContextOwnerError> {
        let lifecycle = self
            .active
            .as_mut()
            .ok_or(ContextOwnerError::NoActiveContext)?;
        let state = lifecycle
            .transition(event)
            .map_err(ContextOwnerError::Lifecycle)?;
        let runtime_transitioned = match event {
            super::lifecycle::LifecycleEvent::Faulted => self.runtime_state.record_fault(),
            super::lifecycle::LifecycleEvent::RecoveryStarted => {
                self.runtime_state.begin_recovery()
            }
            super::lifecycle::LifecycleEvent::Terminated => self.runtime_state.terminate(),
            _ => true,
        };
        if !runtime_transitioned {
            return Err(ContextOwnerError::RuntimeStateMismatch);
        }
        Ok(state)
    }

    /// Retires the active context after the lifecycle reaches `Terminated`.
    pub fn retire(&mut self) -> Result<ActiveContext, ContextOwnerError> {
        if self.active.is_none() {
            return Err(ContextOwnerError::NoActiveContext);
        }
        let context = self.active().ok_or(ContextOwnerError::MissingAllocation)?;
        if !self.runtime_state.is_terminated() {
            return Err(ContextOwnerError::NotTerminated);
        }
        self.active = None;
        self.runtime_state.clear();
        Ok(context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{application::lifecycle::LifecycleEvent, memory::slots::SlotManager};
    use dali_targets::IsolationSlot;

    const SLOT: IsolationSlot = IsolationSlot {
        id: 0,
        name: "slot0",
        code_origin: 0x2000_8000,
        code_length: 0x4000,
        data_origin: 0x2000_C000,
        data_length: 0x4000,
        stack_length: 0x1000,
    };
    static SLOTS: &[IsolationSlot] = &[SLOT];

    fn ready_lifecycle() -> ApplicationLifecycle {
        let mut manager = SlotManager::new(SLOTS).expect("test slot table is valid");
        let allocation = manager.reserve(SLOT).expect("test slot is declared");
        let mut lifecycle = ApplicationLifecycle::discovered(
            ApplicationIdentity::new([1; dali_amrn::v4::PACKAGE_ID_LENGTH]),
            SLOT.id,
        );
        let _ = lifecycle.record_allocation(allocation);
        let _ = lifecycle.transition(LifecycleEvent::Ready);
        lifecycle
    }

    #[test]
    fn owns_only_one_active_context() {
        let state = RuntimeState::new();
        let mut owner = ActiveContextOwner::new(&state);
        let mut first = ready_lifecycle();
        let mut second = ready_lifecycle();
        assert!(owner.activate(&mut first).is_ok());
        assert_eq!(
            owner.activate(&mut second),
            Err(ContextOwnerError::AlreadyActive)
        );
    }

    #[test]
    fn rejects_activation_before_ready_state() {
        let state = RuntimeState::new();
        let mut owner = ActiveContextOwner::new(&state);
        let mut lifecycle = ApplicationLifecycle::discovered(
            ApplicationIdentity::new([1; dali_amrn::v4::PACKAGE_ID_LENGTH]),
            SLOT.id,
        );
        assert_eq!(
            owner.activate(&mut lifecycle),
            Err(ContextOwnerError::InvalidLifecycleState(
                ApplicationState::Discovered
            ))
        );
    }

    #[test]
    fn retires_only_after_terminal_recovery() {
        let state = RuntimeState::new();
        let mut owner = ActiveContextOwner::new(&state);
        let mut lifecycle = ready_lifecycle();
        let _ = owner.activate(&mut lifecycle);
        assert_eq!(owner.retire(), Err(ContextOwnerError::NotTerminated));
        for event in [
            LifecycleEvent::Faulted,
            LifecycleEvent::RecoveryStarted,
            LifecycleEvent::Terminated,
        ] {
            let _ = owner.transition(event);
        }
        assert!(owner.retire().is_ok());
        assert_eq!(owner.active(), None);
    }

    #[test]
    fn tracks_fault_recovery_in_the_kernel_state_channel() {
        let state = RuntimeState::new();
        state.publish_running();
        assert!(state.record_fault());
        assert!(state.begin_recovery());
        assert!(state.terminate());
        assert!(!state.record_fault());
    }
}
