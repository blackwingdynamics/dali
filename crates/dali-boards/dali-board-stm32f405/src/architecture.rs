//! Cortex-M architecture operations for the STM32F405 firmware composition.

use dali_kernel_api::{ArchitectureBackend, ArchitectureOperations, ContextRecord, FaultRegister};

/// Cortex-M implementation of the kernel architecture contract.
pub struct CortexMArchitecture;

const PSP_WORD: usize = 0;
const CONTROL_WORD: usize = 9;
const EXCEPTION_RETURN_WORD: usize = 10;

impl ArchitectureBackend for CortexMArchitecture {
    const SAVED_CONTEXT_LAYOUT: dali_kernel_api::SavedContextLayout =
        dali_kernel_api::SavedContextLayout {
            psp: 0,
            callee_saved: 4,
            control: 36,
            exception_return: 40,
        };

    fn operations() -> ArchitectureOperations {
        ArchitectureOperations {
            wait_for_interrupt: Self::wait_for_interrupt,
            enable_interrupts: Self::enable_interrupts,
            request_context_switch: Self::request_context_switch,
            initial_context: Self::initial_context,
            read_process_stack_pointer: Self::read_process_stack_pointer,
            write_process_stack_pointer: Self::write_process_stack_pointer,
            read_main_stack_pointer: Self::read_main_stack_pointer,
            read_fault_register: Self::read_fault_register,
            write_fault_register: Self::write_fault_register,
            recover_to_kernel: Self::recover_to_kernel,
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

    fn initial_context(psp: u32, exception_return: u32) -> ContextRecord {
        const UNPRIVILEGED_PSP_CONTROL: u32 = 0b11;
        let mut words = [0; dali_kernel_api::CONTEXT_RECORD_WORDS];
        words[PSP_WORD] = psp;
        words[CONTROL_WORD] = UNPRIVILEGED_PSP_CONTROL;
        words[EXCEPTION_RETURN_WORD] = exception_return;
        ContextRecord::new(words)
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

    fn read_fault_register(register: FaultRegister) -> u32 {
        let address = match register {
            FaultRegister::Configurable => 0xE000_ED28_usize,
            FaultRegister::Hard => 0xE000_ED2C_usize,
            FaultRegister::MemoryAddress => 0xE000_ED34_usize,
            FaultRegister::BusAddress => 0xE000_ED38_usize,
            FaultRegister::SystemHandlerControl => 0xE000_ED24_usize,
        };
        // SAFETY: These are fixed, aligned ARMv7-M SCB registers.
        unsafe { core::ptr::read_volatile(address as *const u32) }
    }

    fn write_fault_register(register: FaultRegister, value: u32) {
        if matches!(register, FaultRegister::SystemHandlerControl) {
            // SAFETY: This is the fixed, aligned ARMv7-M SHCSR register.
            unsafe { core::ptr::write_volatile(0xE000_ED24_usize as *mut u32, value) };
        }
    }

    #[cfg(target_arch = "arm")]
    unsafe fn recover_to_kernel(frame_address: u32) -> ! {
        const CONTROL_PRIVILEGED_MSP: u32 = 0;
        const EXC_RETURN_THREAD_MSP_BASIC: u32 = 0xFFFF_FFF9;
        // SAFETY: The kernel supplies an address for a live, validated frame
        // on its own stack and invokes this only during fault recovery.
        unsafe {
            core::arch::asm!(
                "msr MSP, r0",
                "mov r0, {control}",
                "msr CONTROL, r0",
                "isb",
                "mov lr, {exception_return}",
                "bx lr",
                in("r0") frame_address,
                control = const CONTROL_PRIVILEGED_MSP,
                exception_return = const EXC_RETURN_THREAD_MSP_BASIC,
                options(noreturn),
            );
        }
    }

    #[cfg(not(target_arch = "arm"))]
    unsafe fn recover_to_kernel(_frame_address: u32) -> ! {
        loop {
            core::hint::spin_loop();
        }
    }
}
