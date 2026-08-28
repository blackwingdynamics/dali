//! Hardware-neutral CPU architecture operations.

/// Function table installed by the selected firmware composition.
#[derive(Clone, Copy)]
pub struct ArchitectureOperations {
    /// Waits until an interrupt or reset event occurs.
    pub wait_for_interrupt: fn() -> !,
    /// Enables maskable interrupts.
    pub enable_interrupts: fn(),
    /// Requests a deferred context switch.
    pub request_context_switch: fn(),
    /// Reads the process stack pointer.
    pub read_process_stack_pointer: fn() -> u32,
    /// Writes the process stack pointer.
    pub write_process_stack_pointer: unsafe fn(u32),
    /// Reads the main stack pointer.
    pub read_main_stack_pointer: fn() -> u32,
}

/// Low-level operations required by kernel bootstrap and scheduling policy.
///
/// Implementations belong to the selected firmware architecture backend. The
/// kernel consumes this contract without naming a CPU vendor or instruction
/// set.
pub trait ArchitectureBackend {
    /// Returns the architecture operations for kernel runtime use.
    fn operations() -> ArchitectureOperations;

    /// Waits until an interrupt or reset event occurs.
    fn wait_for_interrupt() -> !;

    /// Enables maskable interrupts after bootstrap has installed handlers.
    fn enable_interrupts();

    /// Requests the architecture's deferred context-switch exception.
    fn request_context_switch();
}
