//! Cortex-M architecture operations for the STM32F405 firmware composition.

use dali_kernel_api::ArchitectureBackend;

/// Cortex-M implementation of the kernel architecture contract.
pub struct CortexMArchitecture;

impl ArchitectureBackend for CortexMArchitecture {
    fn wait_for_interrupt() -> ! {
        loop {
            cortex_m::asm::wfi();
        }
    }

    fn enable_interrupts() {
        // SAFETY: Bootstrap calls this only after the vector table and required
        // interrupt handlers have been initialized.
        unsafe { cortex_m::interrupt::enable() };
    }

    fn request_context_switch() {
        cortex_m::peripheral::SCB::set_pendsv();
    }
}
