//! ABI v3 application-side SVC logging call.

#[cfg(target_arch = "arm")]
use crate::{MAX_LOG_MESSAGE_BYTES, svc};

#[cfg(target_arch = "arm")]
pub(super) fn log(message: &str) -> bool {
    if message.len() > MAX_LOG_MESSAGE_BYTES {
        return false;
    }
    let mut status = svc::ServiceId::Log as u32;
    let pointer = message.as_ptr() as usize;
    let length = message.len();
    unsafe {
        // SAFETY: The ABI v3 application owns this immutable message for the
        // duration of the synchronous SVC call and passes only bounded values.
        core::arch::asm!(
            "svc {immediate}",
            immediate = const svc::GATEWAY_IMMEDIATE,
            inout("r0") status,
            in("r1") pointer,
            in("r2") length,
            options(nostack, preserves_flags),
        );
    }
    status == svc::STATUS_OK
}

#[cfg(not(target_arch = "arm"))]
pub(super) fn log(message: &str) -> bool {
    let _ = message;
    false
}
