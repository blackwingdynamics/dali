//! Hardware-neutral CPU architecture operations.

/// Stable layout of the kernel-owned saved context passed to an architecture
/// backend during a context restore.
#[derive(Clone, Copy)]
pub struct SavedContextLayout {
    /// Offset of the saved process stack pointer.
    pub psp: usize,
    /// Offset of the first callee-saved register.
    pub callee_saved: usize,
    /// Offset of the saved CONTROL register.
    pub control: usize,
    /// Offset of the saved exception-return selector.
    pub exception_return: usize,
}

/// Number of machine words reserved by the current context-record contract.
///
/// The selected architecture owns the interpretation and layout of these
/// words. Kernel scheduling policy stores the record without naming registers
/// or exception-return fields.
pub const CONTEXT_RECORD_WORDS: usize = 11;

/// Opaque register record retained by the portable scheduler policy.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContextRecord {
    words: [u32; CONTEXT_RECORD_WORDS],
}

impl ContextRecord {
    /// Creates a record from architecture-port-owned word ordering.
    pub const fn new(words: [u32; CONTEXT_RECORD_WORDS]) -> Self {
        Self { words }
    }

    /// Returns the words for an architecture-owned restore adapter.
    pub const fn words(self) -> [u32; CONTEXT_RECORD_WORDS] {
        self.words
    }
}

/// Fault-status register exposed through an architecture backend.
#[derive(Clone, Copy)]
pub enum FaultRegister {
    /// Configurable fault status.
    Configurable,
    /// Hard-fault status.
    Hard,
    /// Memory-management fault address.
    MemoryAddress,
    /// Bus-fault address.
    BusAddress,
    /// System-handler control and state.
    SystemHandlerControl,
}

/// Function table installed by the selected firmware composition.
#[derive(Clone, Copy)]
pub struct ArchitectureOperations {
    /// Waits until an interrupt or reset event occurs.
    pub wait_for_interrupt: fn() -> !,
    /// Enables maskable interrupts.
    pub enable_interrupts: fn(),
    /// Requests a deferred context switch.
    pub request_context_switch: fn(),
    /// Builds the initial context record for a validated application frame.
    pub initial_context: fn(u32, u32) -> ContextRecord,
    /// Captures an exception-save area into the architecture context record.
    pub capture_context: unsafe fn(*const u32, u32, u32, u32) -> ContextRecord,
    /// Reads the process stack pointer.
    pub read_process_stack_pointer: fn() -> u32,
    /// Writes the process stack pointer.
    pub write_process_stack_pointer: unsafe fn(u32),
    /// Reads the main stack pointer.
    pub read_main_stack_pointer: fn() -> u32,
    /// Reads an architecture fault-status register.
    pub read_fault_register: fn(FaultRegister) -> u32,
    /// Writes an architecture fault-status register when supported.
    pub write_fault_register: fn(FaultRegister, u32),
    /// Returns from a prepared fault-recovery frame.
    pub recover_to_kernel: unsafe fn(u32) -> !,
}

/// Low-level operations required by kernel bootstrap and scheduling policy.
///
/// Implementations belong to the selected firmware architecture backend. The
/// kernel consumes this contract without naming a CPU vendor or instruction
/// set.
pub trait ArchitectureBackend {
    /// Layout of the context record consumed by this architecture's restore
    /// primitive.
    ///
    /// The architecture port owns the record layout. Kernel policy may use
    /// the opaque offsets when passing a record to the port, but must not
    /// name CPU registers or exception-return values.
    const SAVED_CONTEXT_LAYOUT: SavedContextLayout;

    /// Returns the architecture operations for kernel runtime use.
    fn operations() -> ArchitectureOperations;

    /// Waits until an interrupt or reset event occurs.
    fn wait_for_interrupt() -> !;

    /// Enables maskable interrupts after bootstrap has installed handlers.
    fn enable_interrupts();

    /// Requests the architecture's deferred context-switch exception.
    fn request_context_switch();

    /// Builds the initial context record for a validated application frame.
    fn initial_context(psp: u32, exception_return: u32) -> ContextRecord;

    /// Captures an exception-save area into the architecture context record.
    ///
    /// # Safety
    ///
    /// The pointer must reference the complete, aligned save area produced by
    /// the architecture's exception entry wrapper.
    unsafe fn capture_context(
        saved_registers: *const u32,
        psp: u32,
        control: u32,
        exception_return: u32,
    ) -> ContextRecord;
}
