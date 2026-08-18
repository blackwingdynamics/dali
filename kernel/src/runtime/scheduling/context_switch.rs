//! ARM context save/restore primitives for the explicitly enabled scheduler.

#[cfg(target_arch = "arm")]
use super::saved_state::SavedContext;

/// Saves the current Thread-mode state into a kernel-owned context record.
///
/// The caller must execute this from a privileged exception handler and pass a
/// valid, aligned, writable pointer to a kernel-owned [`SavedContext`]. The
/// function does not select another context or return through an exception
/// frame; those responsibilities remain with the scheduler integration.
///
/// # Safety
///
/// The caller must provide a valid writable context pointer and invoke this
/// primitive only from a privileged exception handler with the documented
/// exception-return value.
#[cfg(target_arch = "arm")]
#[unsafe(naked)]
pub unsafe extern "C" fn save_current(context: *mut SavedContext, exception_return: u32) {
    // SAFETY: The caller owns the destination record and invokes this helper
    // from an exception handler where the argument registers are available.
    core::arch::naked_asm!(
        "mrs r2, psp",
        "str r2, [r0, #{psp_offset}]",
        "str r4, [r0, #{r4_offset}]",
        "str r5, [r0, #{r5_offset}]",
        "str r6, [r0, #{r6_offset}]",
        "str r7, [r0, #{r7_offset}]",
        "str r8, [r0, #{r8_offset}]",
        "str r9, [r0, #{r9_offset}]",
        "str r10, [r0, #{r10_offset}]",
        "str r11, [r0, #{r11_offset}]",
        "mrs r2, control",
        "str r2, [r0, #{control_offset}]",
        "str r1, [r0, #{exception_return_offset}]",
        "bx lr",
        psp_offset = const SavedContext::PSP_OFFSET,
        r4_offset = const SavedContext::CALLEE_SAVED_OFFSET,
        r5_offset = const SavedContext::CALLEE_SAVED_OFFSET + 4,
        r6_offset = const SavedContext::CALLEE_SAVED_OFFSET + 8,
        r7_offset = const SavedContext::CALLEE_SAVED_OFFSET + 12,
        r8_offset = const SavedContext::CALLEE_SAVED_OFFSET + 16,
        r9_offset = const SavedContext::CALLEE_SAVED_OFFSET + 20,
        r10_offset = const SavedContext::CALLEE_SAVED_OFFSET + 24,
        r11_offset = const SavedContext::CALLEE_SAVED_OFFSET + 28,
        control_offset = const SavedContext::CONTROL_OFFSET,
        exception_return_offset = const SavedContext::EXCEPTION_RETURN_OFFSET,
    );
}

/// Restores a selected context and returns through its saved exception frame.
///
/// The caller must pass a valid, aligned, readable kernel-owned context whose
/// PSP and exception-return values were validated before this function runs.
///
/// # Safety
///
/// The caller must provide a valid readable context pointer and must have
/// validated its PSP, CONTROL, and exception-return fields before the
/// exception return is performed.
#[cfg(target_arch = "arm")]
#[unsafe(naked)]
pub unsafe extern "C" fn restore_selected(context: *const SavedContext) -> ! {
    // SAFETY: The caller validates the context record, its PSP frame, and its
    // exception-return selector before transferring control here.
    core::arch::naked_asm!(
        "ldr r1, [r0, #{r4_offset}]",
        "mov r4, r1",
        "ldr r1, [r0, #{r5_offset}]",
        "mov r5, r1",
        "ldr r1, [r0, #{r6_offset}]",
        "mov r6, r1",
        "ldr r1, [r0, #{r7_offset}]",
        "mov r7, r1",
        "ldr r1, [r0, #{r8_offset}]",
        "mov r8, r1",
        "ldr r1, [r0, #{r9_offset}]",
        "mov r9, r1",
        "ldr r1, [r0, #{r10_offset}]",
        "mov r10, r1",
        "ldr r1, [r0, #{r11_offset}]",
        "mov r11, r1",
        "ldr r1, [r0, #{psp_offset}]",
        "msr psp, r1",
        "ldr r1, [r0, #{control_offset}]",
        "msr control, r1",
        "isb",
        "ldr lr, [r0, #{exception_return_offset}]",
        "bx lr",
        r4_offset = const SavedContext::CALLEE_SAVED_OFFSET,
        r5_offset = const SavedContext::CALLEE_SAVED_OFFSET + 4,
        r6_offset = const SavedContext::CALLEE_SAVED_OFFSET + 8,
        r7_offset = const SavedContext::CALLEE_SAVED_OFFSET + 12,
        r8_offset = const SavedContext::CALLEE_SAVED_OFFSET + 16,
        r9_offset = const SavedContext::CALLEE_SAVED_OFFSET + 20,
        r10_offset = const SavedContext::CALLEE_SAVED_OFFSET + 24,
        r11_offset = const SavedContext::CALLEE_SAVED_OFFSET + 28,
        psp_offset = const SavedContext::PSP_OFFSET,
        control_offset = const SavedContext::CONTROL_OFFSET,
        exception_return_offset = const SavedContext::EXCEPTION_RETURN_OFFSET,
    );
}
