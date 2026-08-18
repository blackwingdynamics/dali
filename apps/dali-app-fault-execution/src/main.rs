#![no_std]
#![no_main]

use core::panic::PanicInfo;

const TEST_MESSAGE: &str = "Fault injection: execute-never memory";
const TARGET_MEMORY: dali_targets::MemoryProfile = dali_targets::TARGET_F405.memory;

const fn data_probe_address() -> usize {
    match TARGET_MEMORY.isolation {
        Some(isolation) => isolation
            .active_slot()
            .map_or(0, |slot| slot.data_origin as usize),
        None => 0,
    }
}

const DATA_PROBE_ADDRESS: usize = data_probe_address();
const _: () = assert!(DATA_PROBE_ADDRESS != 0);

/// Enters the execute-never fault test after proving SVC communication.
///
/// # Safety
///
/// The kernel invokes this symbol only after validating the ABI v3 package and
/// preparing the application PSP frame.
#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.amiran_entry")]
pub unsafe extern "C" fn amiran_entry() -> ! {
    let _ = dali::log(TEST_MESSAGE);
    let entry: unsafe extern "C" fn() -> ! = unsafe {
        // SAFETY: This function pointer intentionally targets the manifest-
        // declared application data region, which the MPU marks XN.
        core::mem::transmute(DATA_PROBE_ADDRESS | 1)
    };
    unsafe {
        // SAFETY: The call intentionally tests that execution from the XN
        // application-data region is rejected by the MPU.
        entry();
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
