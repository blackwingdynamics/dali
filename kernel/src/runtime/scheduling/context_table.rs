//! Hardware-neutral context-switch records and bounded selection policy.

pub use super::saved_state::{CALLEE_SAVED_REGISTER_COUNT, SavedContext};

/// Identifies one entry in a fixed-capacity context table.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContextId(usize);

impl ContextId {
    /// Creates an identifier for a table entry.
    pub const fn new(index: usize) -> Self {
        Self(index)
    }

    /// Returns the table index represented by this identifier.
    pub const fn index(self) -> usize {
        self.0
    }
}

/// Scheduler-visible state for one saved context.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContextState {
    /// Eligible for selection by the scheduler.
    Ready,
    /// Currently selected for execution.
    Running,
    /// Permanently excluded from selection.
    Terminated,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ContextSlot {
    context: SavedContext,
    state: ContextState,
}

/// Errors returned by the bounded context table.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContextTableError {
    /// No free entry remains in the table.
    Full,
    /// The identifier does not refer to a table entry.
    InvalidId,
    /// A second context cannot become running while one is active.
    ActiveContextExists,
    /// A terminated context cannot be activated.
    TerminatedContext,
}

/// Fixed-capacity context table used by a future PendSV scheduler.
///
/// This table does not perform an exception return or write MPU registers. It
/// only owns the bounded selection contract that those hardware paths consume.
pub struct ContextTable<const CAPACITY: usize> {
    slots: [Option<ContextSlot>; CAPACITY],
    active: Option<ContextId>,
}

impl<const CAPACITY: usize> ContextTable<CAPACITY> {
    /// Creates an empty table without heap allocation.
    pub const fn new() -> Self {
        Self {
            slots: [None; CAPACITY],
            active: None,
        }
    }

    /// Adds a ready context and returns its stable table identifier.
    pub fn insert(&mut self, context: SavedContext) -> Result<ContextId, ContextTableError> {
        let Some((index, slot)) = self
            .slots
            .iter_mut()
            .enumerate()
            .find(|(_, slot)| slot.is_none())
        else {
            return Err(ContextTableError::Full);
        };
        *slot = Some(ContextSlot {
            context,
            state: ContextState::Ready,
        });
        Ok(ContextId::new(index))
    }

    /// Activates one ready context while preserving single-owner semantics.
    pub fn activate(&mut self, id: ContextId) -> Result<(), ContextTableError> {
        if self.active.is_some() && self.active != Some(id) {
            return Err(ContextTableError::ActiveContextExists);
        }
        let slot = self.slot_mut(id)?;
        if slot.state == ContextState::Terminated {
            return Err(ContextTableError::TerminatedContext);
        }
        slot.state = ContextState::Running;
        self.active = Some(id);
        Ok(())
    }

    /// Marks the active context ready after its registers were saved.
    pub fn yield_active(&mut self) -> Result<(), ContextTableError> {
        let Some(id) = self.active else {
            return Err(ContextTableError::InvalidId);
        };
        self.slot_mut(id)?.state = ContextState::Ready;
        self.active = None;
        Ok(())
    }

    /// Terminates one context and removes it from future selection.
    pub fn terminate(&mut self, id: ContextId) -> Result<(), ContextTableError> {
        let slot = self.slot_mut(id)?;
        slot.state = ContextState::Terminated;
        if self.active == Some(id) {
            self.active = None;
        }
        Ok(())
    }

    /// Selects the first ready context after the supplied table position.
    pub fn next_ready(&self, after: Option<ContextId>) -> Option<ContextId> {
        if CAPACITY == 0 {
            return None;
        }
        let start = after.map_or(0, |id| id.index().saturating_add(1));
        (0..CAPACITY)
            .map(|offset| (start + offset) % CAPACITY)
            .find_map(|index| match self.slots[index] {
                Some(ContextSlot {
                    state: ContextState::Ready,
                    ..
                }) => Some(ContextId::new(index)),
                _ => None,
            })
    }

    /// Returns the saved register state for a valid context identifier.
    pub fn get(&self, id: ContextId) -> Result<SavedContext, ContextTableError> {
        Ok(self.slot(id)?.context)
    }

    fn slot(&self, id: ContextId) -> Result<&ContextSlot, ContextTableError> {
        self.slots
            .get(id.index())
            .and_then(Option::as_ref)
            .ok_or(ContextTableError::InvalidId)
    }

    fn slot_mut(&mut self, id: ContextId) -> Result<&mut ContextSlot, ContextTableError> {
        self.slots
            .get_mut(id.index())
            .and_then(Option::as_mut)
            .ok_or(ContextTableError::InvalidId)
    }
}

impl<const CAPACITY: usize> Default for ContextTable<CAPACITY> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONTEXT: SavedContext = SavedContext {
        psp: 0x2000_C000,
        callee_saved: [0; CALLEE_SAVED_REGISTER_COUNT],
        control: 0x03,
        exception_return: 0xFFFF_FFFD,
    };

    #[test]
    fn keeps_the_table_bounded() {
        let mut table = ContextTable::<1>::new();
        assert!(table.insert(CONTEXT).is_ok());
        assert_eq!(table.insert(CONTEXT), Err(ContextTableError::Full));
    }

    #[test]
    fn enforces_one_running_context() {
        let mut table = ContextTable::<2>::new();
        let first = table.insert(CONTEXT).unwrap();
        let second = table.insert(CONTEXT).unwrap();
        assert!(table.activate(first).is_ok());
        assert_eq!(
            table.activate(second),
            Err(ContextTableError::ActiveContextExists)
        );
    }

    #[test]
    fn selects_ready_contexts_round_robin_style() {
        let mut table = ContextTable::<2>::new();
        let first = table.insert(CONTEXT).unwrap();
        let second = table.insert(CONTEXT).unwrap();
        assert_eq!(table.next_ready(None), Some(first));
        assert_eq!(table.next_ready(Some(first)), Some(second));
        table.terminate(second).unwrap();
        assert_eq!(table.next_ready(Some(first)), Some(first));
        table.terminate(first).unwrap();
        assert_eq!(table.next_ready(Some(first)), None);
    }

    #[test]
    fn preserves_saved_register_state() {
        let mut table = ContextTable::<1>::new();
        let context = SavedContext {
            psp: 0x2001_4000,
            callee_saved: [4, 5, 6, 7, 8, 9, 10, 11],
            control: 0x03,
            exception_return: 0xFFFF_FFFD,
        };
        let id = table.insert(context).unwrap();
        assert_eq!(table.get(id), Ok(context));
    }
}
