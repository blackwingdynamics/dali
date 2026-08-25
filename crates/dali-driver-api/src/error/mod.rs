//! Common errors shared by hardware-neutral driver contracts.

/// Failure categories that a bounded driver operation can report.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DriverError {
    /// The requested resource is exclusively owned by another client.
    ResourceBusy,
    /// A pin identifier or pin operation is not valid for the adapter.
    InvalidPin,
    /// The operation exceeded its declared bounded timeout.
    Timeout,
    /// The hardware reported a failure that cannot be classified further.
    HardwareFault,
    /// The requested capability is not implemented by the adapter.
    Unsupported,
    /// The operation is not valid in the adapter's current state.
    InvalidState,
    /// The underlying device is absent or disconnected.
    Disconnected,
    /// The hardware cannot complete the operation without blocking.
    WouldBlock,
    /// The serial peripheral detected a framing error.
    Framing,
    /// The serial peripheral detected a parity error.
    Parity,
    /// The serial peripheral overran its receive boundary.
    Overrun,
    /// A caller-owned buffer cannot represent the requested transfer.
    InvalidBuffer,
    /// A peripheral rejected a transaction with a negative acknowledgment.
    Nack,
    /// A shared bus transaction lost arbitration.
    ArbitrationLost,
    /// The bus reported a protocol or electrical fault.
    BusError,
}

/// Result type used by driver contracts.
pub type DriverResult<T> = Result<T, DriverError>;
