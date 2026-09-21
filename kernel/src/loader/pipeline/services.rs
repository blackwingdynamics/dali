//! Kernel service capability checks for identity-aware cartridge metadata.

use dali_sdk::svc::ServiceId;

/// Bit mask of services exposed by the current ABI gateway.
const SUPPORTED_SERVICE_MASK: u32 = 1u32 << (ServiceId::Log as u32 - 1);

/// Returns whether a cartridge requests only services exposed by this kernel.
pub(crate) const fn supports_required_services(required: u32) -> bool {
    required & !SUPPORTED_SERVICE_MASK == 0
}

#[cfg(test)]
mod tests {
    use super::{SUPPORTED_SERVICE_MASK, supports_required_services};

    #[test]
    fn accepts_the_declared_log_service() {
        assert!(supports_required_services(SUPPORTED_SERVICE_MASK));
    }

    #[test]
    fn rejects_an_undeclared_service_bit() {
        assert!(!supports_required_services(
            SUPPORTED_SERVICE_MASK | (1 << 1)
        ));
    }
}
