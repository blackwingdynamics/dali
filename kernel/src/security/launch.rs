//! Kernel-generated ABI v3 launch-frame validation.

use dali::svc::ExceptionFrame;

const THUMB_STATE_BIT: u32 = 1 << 24;
const EXC_RETURN_THREAD_PSP_BASIC: u32 = 0xFFFF_FFFD;
const NON_RETURNING_LINK: u32 = 0;

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

/// Errors found while constructing the kernel-owned launch context.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LaunchError {
    /// The package entry was not a word-aligned address.
    InvalidEntry,
    /// The declared stack cannot contain a basic exception frame.
    InvalidStack,
}
