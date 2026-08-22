//! Host-testable building blocks for the ABI v3 SVC gateway.
//!
//! These types describe the gateway contract. The default SDK API remains the
//! ABI v2 service table; the feature-gated v3 SDK call is implemented separately.

/// SVC immediate reserved for the Dali application-service gateway.
pub const GATEWAY_IMMEDIATE: u8 = 0;
/// Number of words saved by the processor in the basic Cortex-M exception frame.
pub const BASIC_FRAME_WORDS: usize = 8;
/// Status returned when a service request is accepted.
pub const STATUS_OK: u32 = 0;
/// Status returned when a service request is rejected.
pub const STATUS_REJECTED: u32 = 1;
/// Test-only service identifier used to exercise invalid PSP recovery.
pub const TEST_INVALID_PSP_SERVICE: u32 = 0xFFFF_FF01;
/// Test-only service identifier used to exercise no-frame HardFault recovery.
pub const TEST_NO_FRAME_HARDFAULT_SERVICE: u32 = 0xFFFF_FF02;
/// Test-only service identifier used to verify application DMA denial.
pub const TEST_DMA_REQUEST_SERVICE: u32 = 0xFFFF_FF03;

/// Returns whether a non-null range is fully contained in a declared region.
pub const fn contains_range(
    start: u32,
    length: u32,
    region_start: u32,
    region_length: u32,
) -> bool {
    let end = match start.checked_add(length) {
        Some(end) => end,
        None => return false,
    };
    let region_end = match region_start.checked_add(region_length) {
        Some(end) => end,
        None => return false,
    };
    start != 0 && start >= region_start && end <= region_end
}

/// Versioned service identifiers understood by the gateway.
#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServiceId {
    /// Submit one bounded UTF-8 message to the kernel logger.
    Log = 1,
}

impl ServiceId {
    /// Decodes a raw service identifier without accepting unknown values.
    pub const fn from_raw(value: u32) -> Option<Self> {
        match value {
            value if value == Self::Log as u32 => Some(Self::Log),
            _ => None,
        }
    }
}

/// Basic Cortex-M exception frame supplied by the processor on exception entry.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ExceptionFrame {
    /// General-purpose argument register zero.
    pub r0: u32,
    /// General-purpose argument register one.
    pub r1: u32,
    /// General-purpose argument register two.
    pub r2: u32,
    /// General-purpose argument register three.
    pub r3: u32,
    /// Intra-procedure-call scratch register.
    pub r12: u32,
    /// Link register value saved on exception entry.
    pub lr: u32,
    /// Return program counter saved on exception entry.
    pub pc: u32,
    /// Program status register saved on exception entry.
    pub xpsr: u32,
}

/// Result written to the stacked `r0` slot by a completed service call.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ServiceStatus(pub u32);

impl ServiceStatus {
    /// Creates an accepted service status.
    pub const fn accepted() -> Self {
        Self(STATUS_OK)
    }

    /// Creates a rejected service status.
    pub const fn rejected() -> Self {
        Self(STATUS_REJECTED)
    }

    /// Returns whether the service was accepted.
    pub const fn is_accepted(self) -> bool {
        self.0 == STATUS_OK
    }
}

#[cfg(test)]
mod tests {
    use super::{ExceptionFrame, STATUS_OK, ServiceId, ServiceStatus, contains_range};

    const UNKNOWN_SERVICE_ID: u32 = u32::MAX;
    const REGION_START: u32 = 0x2000_8000;
    const REGION_LENGTH: u32 = 32_768;

    #[test]
    fn decodes_only_declared_service_identifiers() {
        assert_eq!(
            ServiceId::from_raw(ServiceId::Log as u32),
            Some(ServiceId::Log)
        );
        assert_eq!(ServiceId::from_raw(UNKNOWN_SERVICE_ID), None);
    }

    #[test]
    fn models_the_basic_exception_frame() {
        assert_eq!(
            core::mem::size_of::<ExceptionFrame>(),
            core::mem::size_of::<u32>() * 8
        );
    }

    #[test]
    fn distinguishes_accepted_and_rejected_statuses() {
        assert_eq!(ServiceStatus::accepted().0, STATUS_OK);
        assert!(ServiceStatus::accepted().is_accepted());
        assert!(!ServiceStatus::rejected().is_accepted());
    }

    #[test]
    fn validates_bounded_non_null_ranges() {
        assert!(contains_range(REGION_START, 1, REGION_START, REGION_LENGTH));
        assert!(!contains_range(0, 1, REGION_START, REGION_LENGTH));
        assert!(!contains_range(
            REGION_START + REGION_LENGTH,
            1,
            REGION_START,
            REGION_LENGTH
        ));
        assert!(!contains_range(u32::MAX, 1, REGION_START, REGION_LENGTH));
    }
}
