//! Bounded scheduler orchestration for the future PendSV integration.

use super::{
    context_table::{ContextId, ContextTable, ContextTableError, ScheduledContext},
    tick::{TickBudget, TickBudgetError},
};

#[cfg(target_arch = "arm")]
use super::saved_state::ContextRecord;

/// Errors returned by the bounded scheduler facade.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SchedulerError {
    /// The configured tick quantum was invalid.
    Tick(TickBudgetError),
    /// The context table rejected an operation.
    Context(ContextTableError),
    /// No ready context exists for activation.
    NoReadyContext,
    /// No context is currently running.
    NoActiveContext,
    /// The platform could not activate the selected context's protection map.
    ProtectionUnavailable,
}

/// The context transition selected after a completed preemption save.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SwitchSelection {
    /// Context whose registers were saved by PendSV.
    pub outgoing: ContextId,
    /// Context whose registers must be restored by PendSV.
    pub incoming: ContextId,
}

/// Fixed-capacity scheduler state consumed by SysTick and PendSV adapters.
pub struct Scheduler<const CAPACITY: usize> {
    /// Stores the contexts associated with this bounded state.
    contexts: ContextTable<CAPACITY>,
    /// Stores the tick associated with this bounded state.
    tick: TickBudget,
    /// Stores the recovery target associated with this bounded state.
    recovery_target: Option<ContextId>,
}

impl<const CAPACITY: usize> Scheduler<CAPACITY> {
    /// Creates a scheduler with a platform-independent tick quantum.
    pub const fn new(quantum_ticks: u32) -> Result<Self, SchedulerError> {
        let tick = match TickBudget::new(quantum_ticks) {
            Ok(tick) => tick,
            Err(error) => return Err(SchedulerError::Tick(error)),
        };
        Ok(Self {
            contexts: ContextTable::new(),
            tick,
            recovery_target: None,
        })
    }

    /// Adds a ready context to the bounded scheduler table.
    pub fn insert(&mut self, context: ScheduledContext) -> Result<ContextId, SchedulerError> {
        self.contexts
            .insert(context)
            .map_err(SchedulerError::Context)
    }

    /// Activates the first ready context for initial execution.
    pub fn activate_first(&mut self) -> Result<ContextId, SchedulerError> {
        let id = self
            .contexts
            .next_ready(None)
            .ok_or(SchedulerError::NoReadyContext)?;
        self.contexts
            .activate(id)
            .map_err(SchedulerError::Context)?;
        Ok(id)
    }

    /// Records one platform-provided timer tick.
    pub fn on_tick(&mut self) {
        self.tick.on_tick();
    }

    /// Consumes the one-shot preemption request produced by the tick budget.
    pub fn take_preemption_request(&mut self) -> bool {
        self.tick.take_pendsv_request()
    }

    /// Returns whether a pending quantum has a valid ready target.
    pub fn switch_requested(&self) -> bool {
        self.tick.pendsv_requested()
            && self
                .contexts
                .active()
                .and_then(|active| self.contexts.next_ready(Some(active)))
                .is_some()
    }

    /// Saves the active context and selects the next ready context when a
    /// preemption request is pending.
    pub fn prepare_pendsv(
        &mut self,
        saved_context: ScheduledContext,
    ) -> Result<Option<SwitchSelection>, SchedulerError> {
        if !self.take_preemption_request() {
            return Ok(None);
        }
        self.save_active(saved_context)?;
        self.select_next().map(Some)
    }

    /// Replaces the saved state after PendSV captured the outgoing context.
    pub fn save_active(&mut self, context: ScheduledContext) -> Result<ContextId, SchedulerError> {
        let id = self
            .contexts
            .active()
            .ok_or(SchedulerError::NoActiveContext)?;
        self.contexts
            .update(id, context)
            .map_err(SchedulerError::Context)?;
        Ok(id)
    }

    /// Selects the next context after the outgoing context was saved.
    pub fn select_next(&mut self) -> Result<SwitchSelection, SchedulerError> {
        let outgoing = self
            .contexts
            .active()
            .ok_or(SchedulerError::NoActiveContext)?;
        self.contexts
            .yield_active()
            .map_err(SchedulerError::Context)?;
        let incoming = self
            .contexts
            .next_ready(Some(outgoing))
            .ok_or(SchedulerError::NoReadyContext)?;
        self.contexts
            .activate(incoming)
            .map_err(SchedulerError::Context)?;
        Ok(SwitchSelection { outgoing, incoming })
    }

    /// Terminates the active context and selects the next ready context.
    ///
    /// The terminated context is never made ready again. This transition is
    /// used by fault recovery, where there is no valid application register
    /// frame to save before selecting the next context.
    pub fn terminate_active_and_select_next(
        &mut self,
    ) -> Result<Option<SwitchSelection>, SchedulerError> {
        let outgoing = self
            .contexts
            .active()
            .ok_or(SchedulerError::NoActiveContext)?;
        self.contexts
            .terminate(outgoing)
            .map_err(SchedulerError::Context)?;
        let Some(incoming) = self.contexts.next_ready(Some(outgoing)) else {
            return Ok(None);
        };
        self.contexts
            .activate(incoming)
            .map_err(SchedulerError::Context)?;
        Ok(Some(SwitchSelection { outgoing, incoming }))
    }

    /// Arms a selected ready context for restore through the PendSV handler.
    pub fn arm_recovery_restore(&mut self, id: ContextId) -> Result<(), SchedulerError> {
        if self.contexts.active() != Some(id) {
            return Err(SchedulerError::NoActiveContext);
        }
        self.recovery_target = Some(id);
        Ok(())
    }

    /// Consumes a fault-recovery target before normal preemption handling.
    pub fn take_recovery_target(&mut self) -> Option<ContextId> {
        self.recovery_target.take()
    }

    /// Returns the saved state for a selected context.
    pub fn context(&self, id: ContextId) -> Result<ScheduledContext, SchedulerError> {
        self.contexts.get(id).map_err(SchedulerError::Context)
    }

    /// Returns the currently active scheduled context.
    pub fn active_context(&self) -> Result<ScheduledContext, SchedulerError> {
        let id = self
            .contexts
            .active()
            .ok_or(SchedulerError::NoActiveContext)?;
        self.context(id)
    }

    /// Returns a pointer to one selected CPU record for exception restore.
    #[cfg(target_arch = "arm")]
    pub fn context_cpu_ptr(&self, id: ContextId) -> Result<*const ContextRecord, SchedulerError> {
        self.contexts.cpu_ptr(id).map_err(SchedulerError::Context)
    }

    /// Returns a pointer to the active CPU record for a spurious PendSV.
    #[cfg(target_arch = "arm")]
    pub fn active_cpu_ptr(&self) -> Result<*const ContextRecord, SchedulerError> {
        let id = self
            .contexts
            .active()
            .ok_or(SchedulerError::NoActiveContext)?;
        self.context_cpu_ptr(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::scheduling::context_table::{CONTEXT_RECORD_WORDS, ContextRecord};
    use dali_targets::IsolationSlot;

    const SLOT: IsolationSlot = IsolationSlot {
        id: 0,
        name: "test-slot",
        code_origin: 0x1000,
        code_length: 0x4000,
        data_origin: 0x5000,
        data_length: 0x4000,
        stack_length: 0x1000,
    };

    const CONTEXT: ScheduledContext =
        ScheduledContext::new(ContextRecord::new([0; CONTEXT_RECORD_WORDS]), SLOT);

    #[test]
    fn rejects_an_empty_tick_quantum() {
        assert!(matches!(
            Scheduler::<1>::new(0),
            Err(SchedulerError::Tick(TickBudgetError::ZeroQuantum))
        ));
    }

    #[test]
    fn selects_the_next_ready_context_after_a_tick_request() {
        let mut scheduler = Scheduler::<2>::new(1).unwrap();
        let first = scheduler.insert(CONTEXT).unwrap();
        let second = scheduler.insert(CONTEXT).unwrap();
        assert_eq!(scheduler.activate_first(), Ok(first));
        scheduler.on_tick();
        assert!(scheduler.take_preemption_request());
        assert_eq!(
            scheduler.select_next(),
            Ok(SwitchSelection {
                outgoing: first,
                incoming: second,
            })
        );
    }

    #[test]
    fn preserves_the_saved_outgoing_context_before_selection() {
        let mut scheduler = Scheduler::<2>::new(1).unwrap();
        let first = scheduler.insert(CONTEXT).unwrap();
        let _second = scheduler.insert(CONTEXT).unwrap();
        assert_eq!(scheduler.activate_first(), Ok(first));
        let saved = ScheduledContext::new(
            ContextRecord::new([4, 5, 6, 7, 8, 9, 10, 11, 0, 0, 0]),
            SLOT,
        );
        assert_eq!(scheduler.save_active(saved), Ok(first));
        assert_eq!(scheduler.context(first), Ok(saved));
    }

    #[test]
    fn does_not_switch_without_a_preemption_request() {
        let mut scheduler = Scheduler::<1>::new(1).unwrap();
        let first = scheduler.insert(CONTEXT).unwrap();
        assert_eq!(scheduler.activate_first(), Ok(first));
        assert_eq!(scheduler.prepare_pendsv(CONTEXT), Ok(None));
        assert_eq!(scheduler.context(first), Ok(CONTEXT));
    }

    #[test]
    fn does_not_request_switch_without_a_ready_target() {
        let mut scheduler = Scheduler::<1>::new(1).unwrap();
        let first = scheduler.insert(CONTEXT).unwrap();
        assert_eq!(scheduler.activate_first(), Ok(first));
        scheduler.on_tick();
        assert!(!scheduler.switch_requested());
    }

    #[test]
    fn requests_switch_only_with_another_ready_target() {
        let mut scheduler = Scheduler::<2>::new(1).unwrap();
        let first = scheduler.insert(CONTEXT).unwrap();
        let _second = scheduler.insert(CONTEXT).unwrap();
        assert_eq!(scheduler.activate_first(), Ok(first));
        scheduler.on_tick();
        assert!(scheduler.switch_requested());
    }

    #[test]
    fn prepares_save_and_selection_as_one_bounded_transition() {
        let mut scheduler = Scheduler::<2>::new(1).unwrap();
        let first = scheduler.insert(CONTEXT).unwrap();
        let second = scheduler.insert(CONTEXT).unwrap();
        assert_eq!(scheduler.activate_first(), Ok(first));
        scheduler.on_tick();
        let saved = ScheduledContext::new(
            ContextRecord::new([4, 5, 6, 7, 8, 9, 10, 11, 0, 0, 0]),
            SLOT,
        );
        assert_eq!(
            scheduler.prepare_pendsv(saved),
            Ok(Some(SwitchSelection {
                outgoing: first,
                incoming: second,
            }))
        );
        assert_eq!(scheduler.context(first), Ok(saved));
    }

    #[test]
    fn permanently_excludes_faulted_context_before_selecting_next() {
        let mut scheduler = Scheduler::<2>::new(1).unwrap();
        let first = scheduler.insert(CONTEXT).unwrap();
        let second = scheduler.insert(CONTEXT).unwrap();
        assert_eq!(scheduler.activate_first(), Ok(first));

        assert_eq!(
            scheduler.terminate_active_and_select_next(),
            Ok(Some(SwitchSelection {
                outgoing: first,
                incoming: second,
            }))
        );
        assert_eq!(scheduler.active_context(), Ok(CONTEXT));
        assert_eq!(scheduler.arm_recovery_restore(second), Ok(()));
        assert_eq!(scheduler.take_recovery_target(), Some(second));
        assert_eq!(scheduler.take_recovery_target(), None);
        assert_eq!(scheduler.terminate_active_and_select_next(), Ok(None));
        assert_eq!(
            scheduler.active_context(),
            Err(SchedulerError::NoActiveContext)
        );
    }
}
