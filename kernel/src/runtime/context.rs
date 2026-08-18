//! Ownership contract for the single active application context.

use super::{
    lifecycle::{ApplicationIdentity, ApplicationLifecycle, ApplicationState, LifecycleError},
    slots::SlotAllocation,
};

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
    /// The lifecycle being retired does not own the active context.
    IdentityMismatch,
    /// A context can be retired only after terminal recovery.
    NotTerminated,
}

/// Kernel-owned holder for at most one active application context.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ActiveContextOwner {
    active: Option<ActiveContext>,
}

impl ActiveContextOwner {
    /// Creates an owner with no active application context.
    pub const fn new() -> Self {
        Self { active: None }
    }

    /// Returns the currently active context, if one exists.
    pub const fn active(&self) -> Option<ActiveContext> {
        self.active
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
        self.active = Some(context);
        Ok(context)
    }

    /// Retires the active context after the lifecycle reaches `Terminated`.
    pub fn retire(
        &mut self,
        lifecycle: &ApplicationLifecycle,
    ) -> Result<ActiveContext, ContextOwnerError> {
        let context = self.active.ok_or(ContextOwnerError::NoActiveContext)?;
        if context.identity != lifecycle.identity() {
            return Err(ContextOwnerError::IdentityMismatch);
        }
        if lifecycle.state() != ApplicationState::Terminated {
            return Err(ContextOwnerError::NotTerminated);
        }
        self.active = None;
        Ok(context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{lifecycle::LifecycleEvent, slots::SlotManager};
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
        let mut owner = ActiveContextOwner::new();
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
        let mut owner = ActiveContextOwner::new();
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
        let mut owner = ActiveContextOwner::new();
        let mut lifecycle = ready_lifecycle();
        let _ = owner.activate(&mut lifecycle);
        assert_eq!(
            owner.retire(&lifecycle),
            Err(ContextOwnerError::NotTerminated)
        );
        for event in [
            LifecycleEvent::Faulted,
            LifecycleEvent::RecoveryStarted,
            LifecycleEvent::Terminated,
        ] {
            let _ = lifecycle.transition(event);
        }
        assert!(owner.retire(&lifecycle).is_ok());
        assert_eq!(owner.active(), None);
    }
}
