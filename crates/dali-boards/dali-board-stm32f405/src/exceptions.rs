//! Cortex-M exception-entry wrappers for the F405 composition.

/// Selects the hardware-stacked frame and enters the kernel fault policy.
#[unsafe(export_name = "MemoryManagement")]
#[unsafe(naked)]
unsafe extern "C" fn memory_management() {
    // SAFETY: The wrapper selects the active hardware frame and forwards the
    // architecture-provided EXC_RETURN value without modifying either stack.
    core::arch::naked_asm!(
        "tst lr, #4",
        "ite eq",
        "mrseq r0, msp",
        "mrsne r0, psp",
        "mov r1, lr",
        "b dali_kernel_handle_memory_management",
    );
}

/// Selects the hardware-stacked frame and enters the kernel bus-fault policy.
#[unsafe(export_name = "BusFault")]
#[unsafe(naked)]
unsafe extern "C" fn bus_fault() {
    // SAFETY: The wrapper selects the active hardware frame and forwards the
    // architecture-provided EXC_RETURN value without modifying either stack.
    core::arch::naked_asm!(
        "tst lr, #4",
        "ite eq",
        "mrseq r0, msp",
        "mrsne r0, psp",
        "mov r1, lr",
        "b dali_kernel_handle_bus_fault",
    );
}

/// Selects the hardware-stacked frame and enters the kernel usage-fault policy.
#[unsafe(export_name = "UsageFault")]
#[unsafe(naked)]
unsafe extern "C" fn usage_fault() {
    // SAFETY: The wrapper selects the active hardware frame and forwards the
    // architecture-provided EXC_RETURN value without modifying either stack.
    core::arch::naked_asm!(
        "tst lr, #4",
        "ite eq",
        "mrseq r0, msp",
        "mrsne r0, psp",
        "mov r1, lr",
        "b dali_kernel_handle_usage_fault",
    );
}

/// Forwards a no-frame HardFault to the kernel fault policy.
#[unsafe(export_name = "HardFault")]
#[unsafe(naked)]
unsafe extern "C" fn hard_fault() {
    // SAFETY: The wrapper forwards only the processor-supplied EXC_RETURN
    // value; the kernel validates it before reading any stack frame.
    core::arch::naked_asm!("mov r0, lr", "b dali_kernel_handle_hard_fault",);
}

/// Forwards an SVC frame and preserves the exception-return value.
#[unsafe(export_name = "SVCall")]
#[unsafe(naked)]
unsafe extern "C" fn svc() {
    // SAFETY: The wrapper preserves r4 and lr while selecting the stacked frame
    // and returning through the original exception-return value.
    core::arch::naked_asm!(
        "tst lr, #4",
        "ite eq",
        "mrseq r0, msp",
        "mrsne r0, psp",
        "push {{r4, lr}}",
        "mov r4, lr",
        "mov r1, r4",
        "bl dali_kernel_handle_svc",
        "pop {{r4, lr}}",
        "bx lr",
    );
}
