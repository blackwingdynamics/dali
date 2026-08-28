//! Cortex-M architecture operations for the STM32F405 firmware composition.

use dali_kernel_api::{ArchitectureBackend, ArchitectureOperations};

/// Cortex-M implementation of the kernel architecture contract.
pub struct CortexMArchitecture;

impl ArchitectureBackend for CortexMArchitecture {
    fn operations() -> ArchitectureOperations {
        ArchitectureOperations {
            wait_for_interrupt: Self::wait_for_interrupt,
            enable_interrupts: Self::enable_interrupts,
            request_context_switch: Self::request_context_switch,
            read_process_stack_pointer: Self::read_process_stack_pointer,
            write_process_stack_pointer: Self::write_process_stack_pointer,
            read_main_stack_pointer: Self::read_main_stack_pointer,
        }
    }

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

impl CortexMArchitecture {
    fn read_process_stack_pointer() -> u32 {
        cortex_m::register::psp::read()
    }

    unsafe fn write_process_stack_pointer(value: u32) {
        // SAFETY: Callers validate the application stack reservation before
        // changing PSP during a controlled privilege transition.
        unsafe { cortex_m::register::psp::write(value) };
    }

    fn read_main_stack_pointer() -> u32 {
        cortex_m::register::msp::read()
    }
}
