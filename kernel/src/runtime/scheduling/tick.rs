//! Hardware-neutral SysTick budget and PendSV request contract.

/// Errors returned when constructing a bounded scheduler tick budget.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TickBudgetError {
    /// A scheduler quantum must contain at least one tick.
    ZeroQuantum,
}

/// Tracks one preemption quantum without owning a timer peripheral.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TickBudget {
    quantum_ticks: u32,
    elapsed_ticks: u32,
    pendsv_requested: bool,
}

impl TickBudget {
    /// Creates a budget measured in platform-provided SysTick interrupts.
    pub const fn new(quantum_ticks: u32) -> Result<Self, TickBudgetError> {
        if quantum_ticks == 0 {
            return Err(TickBudgetError::ZeroQuantum);
        }
        Ok(Self {
            quantum_ticks,
            elapsed_ticks: 0,
            pendsv_requested: false,
        })
    }

    /// Accounts for one timer tick and requests one deferred context switch
    /// when the configured quantum expires.
    pub fn on_tick(&mut self) {
        self.elapsed_ticks = self.elapsed_ticks.saturating_add(1);
        if self.elapsed_ticks >= self.quantum_ticks {
            self.elapsed_ticks = 0;
            self.pendsv_requested = true;
        }
    }

    /// Takes the one-shot PendSV request produced by the elapsed quantum.
    pub fn take_pendsv_request(&mut self) -> bool {
        let requested = self.pendsv_requested;
        self.pendsv_requested = false;
        requested
    }

    /// Returns whether the current quantum has requested deferred switching.
    pub const fn pendsv_requested(self) -> bool {
        self.pendsv_requested
    }

    /// Returns the number of ticks elapsed in the current quantum.
    pub const fn elapsed_ticks(self) -> u32 {
        self.elapsed_ticks
    }
}

#[cfg(test)]
mod tests {
    use super::{TickBudget, TickBudgetError};

    #[test]
    fn rejects_an_empty_quantum() {
        assert_eq!(TickBudget::new(0), Err(TickBudgetError::ZeroQuantum));
    }

    #[test]
    fn requests_pendsv_once_per_quantum() {
        let mut budget = TickBudget::new(2).unwrap();
        budget.on_tick();
        assert!(!budget.take_pendsv_request());
        budget.on_tick();
        assert!(budget.take_pendsv_request());
        assert!(!budget.take_pendsv_request());
    }

    #[test]
    fn starts_a_new_quantum_after_request() {
        let mut budget = TickBudget::new(2).unwrap();
        budget.on_tick();
        budget.on_tick();
        assert!(budget.take_pendsv_request());
        assert_eq!(budget.elapsed_ticks(), 0);
        budget.on_tick();
        assert_eq!(budget.elapsed_ticks(), 1);
    }
}
