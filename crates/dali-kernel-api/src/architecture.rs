//! Hardware-neutral CPU architecture operations.

/// Low-level operations required by kernel bootstrap and scheduling policy.
///
/// Implementations belong to the selected firmware architecture backend. The
/// kernel consumes this contract without naming a CPU vendor or instruction
/// set.
pub trait ArchitectureBackend {
    /// Waits until an interrupt or reset event occurs.
    fn wait_for_interrupt() -> !;

    /// Enables maskable interrupts after bootstrap has installed handlers.
    fn enable_interrupts();

    /// Requests the architecture's deferred context-switch exception.
    fn request_context_switch();
}
