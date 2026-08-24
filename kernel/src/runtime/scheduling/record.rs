//! Scheduler-owned CPU state and protection binding for one application.

use dali_targets::IsolationSlot;

use super::saved_state::SavedContext;

/// Complete kernel-owned record required to resume one application.
///
/// The slot is retained beside the CPU state so a future privileged PendSV
/// path can select the matching MPU layout before exception return.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScheduledContext {
    /// Stores the cpu associated with this bounded state.
    cpu: SavedContext,
    /// Stores the slot associated with this bounded state.
    slot: IsolationSlot,
}

impl ScheduledContext {
    /// Creates an initial scheduler record for a validated launch frame.
    pub const fn initial(
        psp: u32,
        control: u32,
        exception_return: u32,
        slot: IsolationSlot,
    ) -> Self {
        Self::new(SavedContext::initial(psp, control, exception_return), slot)
    }

    /// Creates a scheduler record from validated CPU state and slot metadata.
    pub const fn new(cpu: SavedContext, slot: IsolationSlot) -> Self {
        Self { cpu, slot }
    }

    /// Returns the saved CPU state consumed by the restore primitive.
    pub const fn cpu(self) -> SavedContext {
        self.cpu
    }

    /// Returns a stable pointer while the owning scheduler is exclusively held.
    #[cfg(target_arch = "arm")]
    pub const fn cpu_ptr(&self) -> *const SavedContext {
        &self.cpu
    }

    /// Returns the manifest-owned slot bound to this execution context.
    pub const fn slot(self) -> IsolationSlot {
        self.slot
    }

    /// Replaces only the saved CPU state after PendSV captures a context.
    pub const fn with_cpu(self, cpu: SavedContext) -> Self {
        Self { cpu, ..self }
    }
}
