//! CPU state preserved across a future PendSV context switch.

/// Number of callee-saved registers preserved by the active context port.
pub const CALLEE_SAVED_REGISTER_COUNT: usize = 8;

/// Kernel-owned CPU state required to resume one application context.
#[repr(C)]
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

impl SavedContext {
    /// Creates the initial CPU record for a validated launch frame.
    pub const fn initial(psp: u32, control: u32, exception_return: u32) -> Self {
        Self {
            psp,
            callee_saved: [0; CALLEE_SAVED_REGISTER_COUNT],
            control,
            exception_return,
        }
    }

    /// Byte offset of the saved PSP field in the ARM assembly layout.
    pub const PSP_OFFSET: usize = core::mem::offset_of!(Self, psp);
    /// Byte offset of the first saved callee-saved register.
    pub const CALLEE_SAVED_OFFSET: usize = core::mem::offset_of!(Self, callee_saved);
    /// Byte offset of the saved CONTROL field.
    pub const CONTROL_OFFSET: usize = core::mem::offset_of!(Self, control);
    /// Byte offset of the saved exception-return selector.
    pub const EXCEPTION_RETURN_OFFSET: usize = core::mem::offset_of!(Self, exception_return);
}
