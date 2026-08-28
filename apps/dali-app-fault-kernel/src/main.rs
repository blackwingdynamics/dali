#![no_std]
#![no_main]

use core::panic::PanicInfo;

const TEST_MESSAGE: &str = "Fault injection: kernel memory read";
const TARGET_MEMORY: dali_targets::MemoryProfile = dali_targets::TARGET_F405.memory;
const KERNEL_PROBE_ADDRESS: usize = TARGET_MEMORY.kernel_origin as usize;

/// Enters the kernel-memory read fault test after proving SVC communication.
///
/// # Safety
///
/// The kernel invokes this symbol only after validating the ABI v3 cartridge and
/// preparing the application PSP frame.
#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.amiran_entry")]
pub unsafe extern "C" fn amiran_entry() -> ! {
    let _ = dali_sdk::log(TEST_MESSAGE);
    let _value = unsafe {
        // SAFETY: This read intentionally targets the manifest-declared kernel
        // region to prove that the unprivileged MPU boundary rejects it.
        core::ptr::read_volatile(KERNEL_PROBE_ADDRESS as *const u32)
    };
    loop {
        core::hint::spin_loop();
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
