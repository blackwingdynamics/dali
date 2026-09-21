#![no_std]
#![no_main]

use core::panic::PanicInfo;

const TEST_MESSAGE: &str = "Fault injection: peripheral memory read";
const TARGET_MEMORY: dali_targets::MemoryProfile = dali_targets::TARGET_F405.memory;

const fn peripheral_probe_address() -> usize {
    match TARGET_MEMORY.isolation {
        Some(isolation) => match isolation.peripheral_origin {
            Some(origin) => origin as usize,
            None => 0,
        },
        None => 0,
    }
}

const PERIPHERAL_PROBE_ADDRESS: usize = peripheral_probe_address();
const _: () = assert!(PERIPHERAL_PROBE_ADDRESS != 0);

/// Enters the peripheral-access fault test after proving SVC communication.
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
        // SAFETY: This read intentionally targets the manifest-declared
        // peripheral region to prove that unprivileged MMIO access is rejected.
        core::ptr::read_volatile(PERIPHERAL_PROBE_ADDRESS as *const u32)
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
