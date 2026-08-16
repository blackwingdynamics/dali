//! Kernel-owned fault context for the planned isolated application boundary.
//!
//! This module defines the bounded record used by future exception handlers. It
//! does not install handlers or attempt recovery while ABI v2 remains active.

use crate::logging;

/// Fault sources that must terminate an isolated application.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FaultKind {
    /// The exception return value did not identify the expected app context.
    InvalidExceptionReturn,
}

/// Bounded context captured before an isolated application is terminated.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct FaultRecord {
    /// Fault source.
    pub(crate) kind: FaultKind,
    /// Stacked program counter, when a valid basic frame was available.
    pub(crate) stacked_pc: Option<u32>,
    /// Stacked link register, when a valid basic frame was available.
    pub(crate) stacked_lr: Option<u32>,
    /// Fault address, when the architecture reported one.
    pub(crate) fault_address: Option<u32>,
    /// Architecture status bits associated with the fault.
    pub(crate) status: u32,
}

impl FaultRecord {
    /// Creates a record for a fault without a validated application frame.
    pub(crate) const fn without_frame(kind: FaultKind, status: u32) -> Self {
        Self {
            kind,
            stacked_pc: None,
            stacked_lr: None,
            fault_address: None,
            status,
        }
    }
}

/// Reports a bounded fault record without attempting recovery.
pub(crate) fn report(record: FaultRecord) {
    logging::error(
        logging::SECURITY_SUBSYSTEM,
        format_args!(
            "[SECURITY][FAULT] kind={:?} status=0x{:08X} pc={:?} lr={:?} address={:?}",
            record.kind, record.status, record.stacked_pc, record.stacked_lr, record.fault_address
        ),
    );
}
