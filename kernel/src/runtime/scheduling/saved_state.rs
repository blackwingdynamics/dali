//! CPU state preserved across a future PendSV context switch.

/// Number of ARM callee-saved registers preserved by a context switch.
pub const CALLEE_SAVED_REGISTER_COUNT: usize = 8;

/// Kernel-owned CPU state required to resume one application context.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SavedContext {
    /// Process stack pointer used by the exception return.
    pub psp: u32,
    /// ARM `r4..r11` callee-saved registers.
    pub callee_saved: [u32; CALLEE_SAVED_REGISTER_COUNT],
    /// Thread privilege and stack-selection state.
    pub control: u32,
    /// Exception-return selector for the prepared context.
    pub exception_return: u32,
}
