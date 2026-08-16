//! Kernel-generated ABI v3 launch-frame validation.

use dali::svc::ExceptionFrame;

const THUMB_STATE_BIT: u32 = 1 << 24;
const EXC_RETURN_THREAD_PSP_BASIC: u32 = 0xFFFF_FFFD;
const NON_RETURNING_LINK: u32 = 0;
#[cfg(feature = "abi-v3-mpu")]
const CONTROL_UNPRIVILEGED_PSP: u32 = 0b11;

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
#[cfg(feature = "abi-v3-mpu")]
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

/// Returns from the privileged PendSV handler into the prepared PSP frame.
#[cfg(feature = "abi-v3-mpu")]
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
