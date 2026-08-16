#![no_std]
#![no_main]

use core::panic::PanicInfo;

const TEST_MESSAGE: &str = "Fault injection: BusFault address";
const TARGET_MEMORY: dali_targets::MemoryProfile = dali_targets::TARGET_F405.memory;

const fn bus_fault_probe_address() -> usize {
    match TARGET_MEMORY.isolation {
        Some(isolation) => match isolation.bus_fault_origin {
            Some(origin) => origin as usize,
            None => 0,
        },
        None => 0,
    }
}

const BUS_FAULT_PROBE_ADDRESS: usize = bus_fault_probe_address();
const _: () = assert!(BUS_FAULT_PROBE_ADDRESS != 0);

/// Enters the deterministic BusFault address test.
///
/// # Safety
///
/// The kernel invokes this symbol only after validating the ABI v3 package and
/// preparing the application PSP frame.
#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.amiran_entry")]
pub unsafe extern "C" fn amiran_entry() -> ! {
    let _ = dali::log_v3(TEST_MESSAGE);
    let _value = unsafe {
        // SAFETY: This read intentionally targets the address declared by the
        // F405 manifest to produce a precise processor bus fault.
        core::ptr::read_volatile(BUS_FAULT_PROBE_ADDRESS as *const u32)
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
