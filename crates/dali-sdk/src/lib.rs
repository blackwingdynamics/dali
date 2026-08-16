#![no_std]

//! Dali OS application SDK and kernel-service ABI.

pub mod svc;

/// Maximum UTF-8 message length accepted by the initial application logger.
pub const MAX_LOG_MESSAGE_BYTES: usize = 96;
/// Return code for a service call that was accepted by the kernel.
pub const LOG_OK: u32 = 0;
/// Return code for a service call rejected by the kernel.
pub const LOG_REJECTED: u32 = 1;

/// Kernel services passed to an AMRN application at its entry point.
#[repr(C)]
pub struct ServiceTable {
    /// Submit one bounded UTF-8 message to the kernel logging facade.
    pub log: unsafe extern "C" fn(*const u8, usize) -> u32,
}

/// Submits a message through the kernel-owned application logging service.
pub fn log(services: &ServiceTable, message: &str) -> bool {
    if message.len() > MAX_LOG_MESSAGE_BYTES {
        return false;
    }
    let result = unsafe {
        // SAFETY: The kernel provides the service table and its function pointer
        // for the lifetime of the non-returning application entry point.
        (services.log)(message.as_ptr(), message.len())
    };
    result == LOG_OK
}

#[cfg(test)]
mod tests {
    use super::{LOG_OK, LOG_REJECTED, MAX_LOG_MESSAGE_BYTES, ServiceTable, log};

    unsafe extern "C" fn accept_log(_message: *const u8, _length: usize) -> u32 {
        LOG_OK
    }

    unsafe extern "C" fn reject_log(_message: *const u8, _length: usize) -> u32 {
        LOG_REJECTED
    }

    #[test]
    fn forwards_bounded_messages_to_the_kernel_service() {
        let services = ServiceTable { log: accept_log };

        assert!(log(&services, "Hello World from AMRN"));
    }

    #[test]
    fn rejects_messages_over_the_contract_limit_before_calling_kernel() {
        let services = ServiceTable { log: reject_log };
        let message = "x".repeat(MAX_LOG_MESSAGE_BYTES + 1);

        assert!(!log(&services, &message));
    }

    #[test]
    fn propagates_kernel_rejection() {
        let services = ServiceTable { log: reject_log };

        assert!(!log(&services, "rejected"));
    }
}
