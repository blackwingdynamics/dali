#![no_std]
#![no_main]

use core::panic::PanicInfo;

const TEST_MESSAGE: &str = "Fault injection: invalid PSP bounds";
const TARGET_MEMORY: dali_targets::MemoryProfile = dali_targets::TARGET_F405.memory;

const fn invalid_psp() -> usize {
    match TARGET_MEMORY.isolation {
        Some(isolation) => isolation.data_origin as usize + 4,
        None => 0,
    }
}

const INVALID_PSP: usize = invalid_psp();
const _: () = assert!(INVALID_PSP != 0);
const INVALID_PSP_LOW: u32 = (INVALID_PSP as u32) & 0xFFFF;
const INVALID_PSP_HIGH: u32 = (INVALID_PSP as u32) >> 16;

/// Enters the invalid-PSP fault test after proving SVC communication.
///
/// # Safety
///
/// The kernel invokes this symbol only after validating the ABI v3 package and
/// preparing the application PSP frame.
#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.amiran_entry")]
pub unsafe extern "C" fn amiran_entry() -> ! {
    let _ = dali::log_v3(TEST_MESSAGE);
    unsafe { trigger_invalid_psp() }
}

#[unsafe(naked)]
unsafe extern "C" fn trigger_invalid_psp() -> ! {
    // SAFETY: This test intentionally selects a PSP value four bytes inside
    // the declared data boundary, so SVC exception stacking crosses it.
    core::arch::naked_asm!(
        "movw r0, {invalid_psp_low}",
        "movt r0, {invalid_psp_high}",
        "msr psp, r0",
        "svc 0",
        "b .",
        invalid_psp_low = const INVALID_PSP_LOW,
        invalid_psp_high = const INVALID_PSP_HIGH,
    );
}

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
