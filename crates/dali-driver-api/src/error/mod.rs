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
}

/// Result type used by driver contracts.
pub type DriverResult<T> = Result<T, DriverError>;
