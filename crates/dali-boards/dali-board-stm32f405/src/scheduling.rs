//! Cortex-M context-switch exception entry for the F405 backend.

#[cfg(feature = "abi-context-switch")]
use dali_kernel_api::ArchitectureBackend;

#[cfg(feature = "abi-context-switch")]
use crate::architecture::CortexMArchitecture;

#[cfg(feature = "abi-context-switch")]
const CONTEXT_PSP_OFFSET: usize = CortexMArchitecture::SAVED_CONTEXT_LAYOUT.psp;
#[cfg(feature = "abi-context-switch")]
const CONTEXT_CALLEE_SAVED_OFFSET: usize = CortexMArchitecture::SAVED_CONTEXT_LAYOUT.callee_saved;
#[cfg(feature = "abi-context-switch")]
const CONTEXT_CONTROL_OFFSET: usize = CortexMArchitecture::SAVED_CONTEXT_LAYOUT.control;
#[cfg(feature = "abi-context-switch")]
const CONTEXT_EXCEPTION_RETURN_OFFSET: usize =
    CortexMArchitecture::SAVED_CONTEXT_LAYOUT.exception_return;

#[cfg(all(target_arch = "arm", feature = "abi-context-switch"))]
#[unsafe(export_name = "PendSV")]
#[unsafe(naked)]
unsafe extern "C" fn pendsv_handler() -> ! {
    core::arch::naked_asm!(
        "stmdb sp!, {{r4-r11}}",
        "mrs r1, psp",
        "mrs r2, control",
        "mov r3, lr",
        "mov r0, sp",
        "bl dali_kernel_prepare_pendsv",
        "add sp, #32",
        "b {restore}",
        restore = sym restore_selected,
    );
}

/// Restores the selected context after the kernel has chosen the next task.
///
/// Keeping the restore sequence in a separate naked function preserves the
/// exception-return boundary used by the original context-switch backend.
#[cfg(all(target_arch = "arm", feature = "abi-context-switch"))]
#[unsafe(naked)]
unsafe extern "C" fn restore_selected() -> ! {
    core::arch::naked_asm!(
        "ldr r1, [r0, #{callee_saved}]",
        "mov r4, r1",
        "ldr r1, [r0, #{callee_saved}+4]",
        "mov r5, r1",
        "ldr r1, [r0, #{callee_saved}+8]",
        "mov r6, r1",
        "ldr r1, [r0, #{callee_saved}+12]",
        "mov r7, r1",
        "ldr r1, [r0, #{callee_saved}+16]",
        "mov r8, r1",
        "ldr r1, [r0, #{callee_saved}+20]",
        "mov r9, r1",
        "ldr r1, [r0, #{callee_saved}+24]",
        "mov r10, r1",
        "ldr r1, [r0, #{callee_saved}+28]",
        "mov r11, r1",
        "ldr r1, [r0, #{psp}]",
        "msr psp, r1",
        "ldr r1, [r0, #{control}]",
        "msr control, r1",
        "isb",
        "ldr lr, [r0, #{exception_return}]",
        "bx lr",
        callee_saved = const CONTEXT_CALLEE_SAVED_OFFSET,
        psp = const CONTEXT_PSP_OFFSET,
        control = const CONTEXT_CONTROL_OFFSET,
        exception_return = const CONTEXT_EXCEPTION_RETURN_OFFSET,
    );
}

#[cfg(all(target_arch = "arm", not(feature = "abi-context-switch")))]
#[unsafe(export_name = "PendSV")]
#[unsafe(naked)]
unsafe extern "C" fn launch_handler() -> ! {
    const CONTROL_UNPRIVILEGED_PSP: u32 = 0b11;
    const EXC_RETURN_THREAD_PSP_BASIC: u32 = 0xFFFF_FFFD;

    core::arch::naked_asm!(
        "mov r0, {control}",
        "msr CONTROL, r0",
        "isb",
        "mov lr, {exception_return}",
        "bx lr",
        control = const CONTROL_UNPRIVILEGED_PSP,
        exception_return = const EXC_RETURN_THREAD_PSP_BASIC,
    );
}
