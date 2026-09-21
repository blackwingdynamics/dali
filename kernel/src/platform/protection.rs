//! Registered memory-protection operations supplied by a selected backend.

use dali_kernel_api::BoardBackend;

/// Memory-protection operations registered by the selected backend.
static MEMORY_PROTECTION: critical_section::Mutex<
    core::cell::RefCell<Option<dali_kernel_api::MemoryProtectionOperations>>,
> = critical_section::Mutex::new(core::cell::RefCell::new(None));

/// Registers the selected backend protection operations once.
pub(super) fn register<B: BoardBackend>() {
    critical_section::with(|cs| {
        let mut protection = MEMORY_PROTECTION.borrow(cs).borrow_mut();
        if protection.is_none() {
            *protection = B::memory_protection_operations();
        }
    });
}

/// Configures the initial protection map through the selected backend.
pub(crate) fn configure(memory: dali_targets::MemoryProfile) -> bool {
    let Some(operations) = critical_section::with(|cs| *MEMORY_PROTECTION.borrow(cs).borrow())
    else {
        return false;
    };
    (operations.configure)(memory);
    true
}

/// Activates application permissions through the selected backend.
pub(crate) fn activate_application_regions(slot: dali_targets::IsolationSlot) -> bool {
    let Some(operations) = critical_section::with(|cs| *MEMORY_PROTECTION.borrow(cs).borrow())
    else {
        return false;
    };
    (operations.activate_application_regions)(slot)
}
