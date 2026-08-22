//! Storage lifecycle state and event vocabulary.

/// Observable states for a bounded storage medium lifecycle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorageLifecycleState {
    /// No storage presence has been established.
    Unavailable,
    /// A storage medium is being probed or has been detected.
    Present,
    /// The medium completed initialization and can serve operations.
    Ready,
    /// The medium stopped responding during an operation.
    Removed,
    /// The medium or transport reported a non-removal failure.
    Fault,
}

/// Events consumed by the storage lifecycle state machine.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorageLifecycleEvent {
    /// A bounded initialization or reinitialization attempt started.
    InitializationStarted,
    /// Initialization completed successfully.
    InitializationSucceeded,
    /// The transport reported that the medium stopped responding.
    CardRemoved,
    /// An operation failed without a removal indication.
    OperationFailed,
    /// A ready-state operation completed successfully.
    OperationSucceeded,
}
