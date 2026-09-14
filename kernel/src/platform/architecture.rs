//! Registered architecture operations used by kernel policy.

use dali_kernel_api::{ArchitectureBackend, BoardBackend, FaultRegister};

/// Architecture operations registered by the selected firmware composition.
static ARCHITECTURE: critical_section::Mutex<
    core::cell::RefCell<Option<dali_kernel_api::ArchitectureOperations>>,
> = critical_section::Mutex::new(core::cell::RefCell::new(None));

/// Registers the selected architecture operations once.
pub(super) fn register<B: BoardBackend>() {
    critical_section::with(|cs| {
        let mut architecture = ARCHITECTURE.borrow(cs).borrow_mut();
        if architecture.is_none() {
            *architecture = Some(B::Architecture::operations());
        }
    });
}

/// Requests a deferred context switch through the installed architecture.
#[cfg(any(feature = "abi-context-switch", feature = "abi-mpu"))]
pub(crate) fn request_context_switch() {
    if let Some(operations) = critical_section::with(|cs| *ARCHITECTURE.borrow(cs).borrow()) {
        (operations.request_context_switch)();
    }
}

/// Waits through the installed architecture after bootstrap registration.
#[cfg(feature = "abi-current")]
pub(crate) fn wait_for_registered_interrupt() -> ! {
    if let Some(operations) = critical_section::with(|cs| *ARCHITECTURE.borrow(cs).borrow()) {
        (operations.wait_for_interrupt)();
    }
    loop {
        core::hint::spin_loop();
    }
}

/// Reads the registered architecture process stack pointer.
#[cfg(feature = "abi-current")]
pub(crate) fn process_stack_pointer() -> Option<u32> {
    critical_section::with(|cs| {
        ARCHITECTURE
            .borrow(cs)
            .borrow()
            .map(|operations| (operations.read_process_stack_pointer)())
    })
}

/// Writes the registered architecture process stack pointer.
#[cfg(feature = "abi-current")]
pub(crate) unsafe fn set_process_stack_pointer(value: u32) -> bool {
    let Some(operations) = critical_section::with(|cs| *ARCHITECTURE.borrow(cs).borrow()) else {
        return false;
    };
    // SAFETY: The caller validates the target stack invariant.
    unsafe { (operations.write_process_stack_pointer)(value) };
    true
}

/// Reads the registered architecture main stack pointer.
#[cfg(feature = "abi-current")]
pub(crate) fn main_stack_pointer() -> Option<u32> {
    critical_section::with(|cs| {
        ARCHITECTURE
            .borrow(cs)
            .borrow()
            .map(|operations| (operations.read_main_stack_pointer)())
    })
}

/// Reads a fault register through the selected architecture backend.
#[inline]
pub(crate) fn read_fault_register(register: FaultRegister) -> u32 {
    critical_section::with(|cs| {
        ARCHITECTURE
            .borrow(cs)
            .borrow()
            .map_or(0, |operations| (operations.read_fault_register)(register))
    })
}

/// Writes a fault register through the selected architecture backend.
#[cfg(feature = "abi-test-fixtures")]
pub(crate) fn write_fault_register(register: FaultRegister, value: u32) {
    if let Some(operations) = critical_section::with(|cs| *ARCHITECTURE.borrow(cs).borrow()) {
        (operations.write_fault_register)(register, value);
    }
}

/// Returns from a prepared fault-recovery frame through the backend.
///
/// # Safety
///
/// `frame_address` must point to a validated kernel-owned recovery frame.
#[inline(never)]
pub(crate) unsafe fn recover_to_kernel(frame_address: u32) -> ! {
    if let Some(operations) = critical_section::with(|cs| *ARCHITECTURE.borrow(cs).borrow()) {
        // SAFETY: The caller has built a validated kernel-owned recovery frame.
        unsafe { (operations.recover_to_kernel)(frame_address) };
    }
    loop {
        core::hint::spin_loop();
    }
}
