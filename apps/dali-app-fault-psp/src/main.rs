#![no_std]
#![no_main]

use core::panic::PanicInfo;

const TEST_MESSAGE: &str = "Fault injection: invalid PSP bounds";
const TEST_INVALID_PSP_SERVICE: u32 = dali_sdk::svc::TEST_INVALID_PSP_SERVICE;

/// Enters the invalid-PSP fault test after proving SVC communication.
///
/// # Safety
///
/// The kernel invokes this symbol only after validating the ABI v3 cartridge and
/// preparing the application PSP frame.
#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.amiran_entry")]
pub unsafe extern "C" fn amiran_entry() -> ! {
    let _ = dali_sdk::log(TEST_MESSAGE);
    let _ = unsafe { trigger_invalid_psp() };
    loop {
        core::hint::spin_loop();
    }
}

unsafe fn trigger_invalid_psp() -> u32 {
    let mut status = TEST_INVALID_PSP_SERVICE;
    unsafe {
        // SAFETY: This non-production fixture requests the kernel-owned test
        // service through the same bounded SVC gateway as normal services.
        core::arch::asm!(
            "svc {immediate}",
            immediate = const dali_sdk::svc::GATEWAY_IMMEDIATE,
            inout("r0") status,
            options(nostack, preserves_flags),
        );
    }
    status
}

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
