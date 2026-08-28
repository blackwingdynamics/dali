//! Cortex-M context-switch exception entry for the F405 backend.

const CONTEXT_PSP_OFFSET: usize = dali_kernel_api::ARMV7M_SAVED_CONTEXT.psp;
const CONTEXT_CALLEE_SAVED_OFFSET: usize = dali_kernel_api::ARMV7M_SAVED_CONTEXT.callee_saved;
const CONTEXT_CONTROL_OFFSET: usize = dali_kernel_api::ARMV7M_SAVED_CONTEXT.control;
const CONTEXT_EXCEPTION_RETURN_OFFSET: usize =
    dali_kernel_api::ARMV7M_SAVED_CONTEXT.exception_return;

#[cfg(target_arch = "arm")]
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
