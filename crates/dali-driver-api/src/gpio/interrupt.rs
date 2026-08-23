//! GPIO interrupt contracts.

use crate::DriverResult;

/// Edge or level condition used by an interrupt-capable input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InterruptTrigger {
    /// Trigger on a low-to-high transition.
    RisingEdge,
    /// Trigger on a high-to-low transition.
    FallingEdge,
    /// Trigger on either transition.
    BothEdges,
    /// Trigger while the input is high.
    LevelHigh,
    /// Trigger while the input is low.
    LevelLow,
}

/// Interrupt capability for a GPIO input.
pub trait InterruptPin {
    /// Enables one bounded trigger condition.
    fn enable_interrupt(&mut self, trigger: InterruptTrigger) -> DriverResult<()>;

    /// Disables GPIO interrupt delivery.
    fn disable_interrupt(&mut self) -> DriverResult<()>;

    /// Reports and clears whether an interrupt is pending.
    fn take_pending(&mut self) -> DriverResult<bool>;
}
