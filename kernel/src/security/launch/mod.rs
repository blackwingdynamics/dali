//! Kernel-generated ABI v3 launch-frame validation.

use dali_sdk::svc::ExceptionFrame;

/// xPSR bit required for a Thumb-state application frame.
const THUMB_STATE_BIT: u32 = 1 << 24;
/// Exception return selector for an unprivileged PSP basic frame.
const EXC_RETURN_THREAD_PSP_BASIC: u32 = 0xFFFF_FFFD;
/// Exception return selector for a privileged MSP basic frame.
const EXC_RETURN_THREAD_MSP_BASIC: u32 = 0xFFFF_FFF9;
/// Link value used by non-returning synthetic frames.
const NON_RETURNING_LINK: u32 = 0;
#[cfg(all(feature = "abi-mpu", not(feature = "abi-context-switch")))]
/// CONTROL value selecting unprivileged Thread mode with PSP.
const CONTROL_UNPRIVILEGED_PSP: u32 = 0b11;
/// CONTROL value selecting privileged Thread mode with MSP.
const CONTROL_PRIVILEGED_MSP: u32 = 0;

/// A validated basic exception frame and its architectural return selector.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct LaunchFrame {
    /// Address where the kernel will place the frame in the application PSP.
    pub(crate) frame_address: u32,
    /// Initial PSP value used by the exception return.
    pub(crate) psp: u32,
    /// Kernel-generated exception return selector.
    pub(crate) exception_return: u32,
    /// Basic frame popped by the exception return.
    pub(crate) frame: ExceptionFrame,
}

/// Builds the first unprivileged application frame without entering it.
pub(crate) fn prepare(
    entry_address: u32,
    stack_origin: u32,
    stack_size: u32,
) -> Result<LaunchFrame, LaunchError> {
    if entry_address & 1 != 0 {
        return Err(LaunchError::InvalidEntry);
    }
    let stack_top = stack_origin
        .checked_add(stack_size)
        .ok_or(LaunchError::InvalidStack)?;
    let frame_bytes = u32::try_from(core::mem::size_of::<ExceptionFrame>())
        .map_err(|_| LaunchError::InvalidStack)?;
    let frame_address = stack_top
        .checked_sub(frame_bytes)
        .ok_or(LaunchError::InvalidStack)?;
    if stack_size == 0 || frame_address < stack_origin || frame_address & 3 != 0 {
        return Err(LaunchError::InvalidStack);
    }

    Ok(LaunchFrame {
        frame_address,
        psp: frame_address,
        exception_return: EXC_RETURN_THREAD_PSP_BASIC,
        frame: ExceptionFrame {
            lr: NON_RETURNING_LINK,
            pc: entry_address | 1,
            xpsr: THUMB_STATE_BIT,
            ..ExceptionFrame::default()
        },
    })
}

/// Materializes the validated frame in the application PSP reservation.
pub(crate) fn materialize(frame: LaunchFrame) {
    unsafe {
        // SAFETY: `LaunchFrame` can only be constructed by `prepare`, which
        // proves that the destination is aligned and fully inside the declared
        // application stack reservation.
        core::ptr::write(frame.frame_address as *mut ExceptionFrame, frame.frame);
    }
}

/// Transfers control through PendSV into one prepared unprivileged context.
#[cfg(feature = "abi-mpu")]
pub(crate) fn enter(frame: LaunchFrame) -> ! {
    materialize(frame);
    unsafe {
        // SAFETY: `prepare` validated the PSP frame address and the kernel is
        // still using MSP, so writing PSP cannot corrupt the active kernel stack.
        cortex_m::register::psp::write(frame.psp);
    }
    cortex_m::peripheral::SCB::set_pendsv();
    loop {
        cortex_m::asm::wfi();
    }
}

/// Returns from an application fault to a kernel-owned recovery loop.
pub(crate) fn recover() -> ! {
    let frame = ExceptionFrame {
        lr: NON_RETURNING_LINK,
        pc: (fault_recovery as *const () as usize as u32) | 1,
        xpsr: THUMB_STATE_BIT,
        ..ExceptionFrame::default()
    };
    let frame_address = (&frame as *const ExceptionFrame) as usize as u32;
    unsafe {
        // SAFETY: `frame` is a live kernel-stack object. This non-returning
        // sequence changes MSP to that object, restores privileged Thread mode,
        // and immediately exception-returns before the object can be dropped.
        core::arch::asm!(
            "msr MSP, {frame_address}",
            "mov r0, {control}",
            "msr CONTROL, r0",
            "isb",
            "mov lr, {exception_return}",
            "bx lr",
            frame_address = in(reg) frame_address,
            control = const CONTROL_PRIVILEGED_MSP,
            exception_return = const EXC_RETURN_THREAD_MSP_BASIC,
            options(noreturn),
        );
    }
}

/// Terminates the failed application and enters the configured recovery path.
extern "C" fn fault_recovery() -> ! {
    if crate::runtime::application::owner::begin_active_recovery() {
        crate::logging::info(
            crate::logging::SECURITY_SUBSYSTEM,
            format_args!("[SECURITY][FAULT] Active context state: Recovering"),
        );
    }
    crate::logging::error(
        crate::logging::SECURITY_SUBSYSTEM,
        format_args!("[SECURITY][FAULT] Application terminated; kernel recovery active"),
    );
    if crate::runtime::application::owner::terminate_active_context() {
        crate::logging::info(
            crate::logging::SECURITY_SUBSYSTEM,
            format_args!("[SECURITY][FAULT] Active context state: Terminated"),
        );
    }
    #[cfg(all(feature = "abi-context-switch", target_arch = "arm"))]
    crate::security::scheduling::recover_faulted_context();
    #[cfg(not(all(feature = "abi-context-switch", target_arch = "arm")))]
    match crate::runtime::application::policy::CURRENT.fault_recovery() {
        crate::runtime::application::policy::FaultRecoveryAction::EnterKernelHeartbeat => loop {
            cortex_m::asm::wfi();
        },
    }
}

/// Returns from the privileged PendSV handler into the prepared PSP frame.
#[cfg(all(feature = "abi-mpu", not(feature = "abi-context-switch")))]
#[unsafe(export_name = "PendSV")]
unsafe extern "C" fn pendsv_handler() -> ! {
    unsafe {
        // SAFETY: PendSV runs in privileged Handler mode. The kernel selected
        // the validated PSP before setting PENDSVSET and owns this transition.
        core::arch::asm!(
            "mov r0, {control}",
            "msr CONTROL, r0",
            "isb",
            "mov lr, {exception_return}",
            "bx lr",
            control = const CONTROL_UNPRIVILEGED_PSP,
            exception_return = const EXC_RETURN_THREAD_PSP_BASIC,
            options(noreturn),
        );
    }
}

/// Errors found while constructing the kernel-owned launch context.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LaunchError {
    /// The package entry was not a word-aligned address.
    InvalidEntry,
    /// The declared stack cannot contain a basic exception frame.
    InvalidStack,
}
