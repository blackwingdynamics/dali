#![no_std]
#![no_main]

use core::panic::PanicInfo;

const TEST_MESSAGE: &str = "Fault injection: no-frame HardFault";
const TEST_SERVICE: u32 = dali_sdk::svc::TEST_NO_FRAME_HARDFAULT_SERVICE;

/// Requests the kernel-owned no-frame HardFault fixture.
///
/// # Safety
///
/// The kernel invokes this symbol only after validating the ABI v3 cartridge and
/// preparing the application PSP frame.
#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.amiran_entry")]
pub unsafe extern "C" fn amiran_entry() -> ! {
    let _ = dali_sdk::log(TEST_MESSAGE);
    let mut status = TEST_SERVICE;
    unsafe {
        // SAFETY: This non-production fixture requests the kernel-owned
        // no-frame fault path through the bounded SVC gateway.
        core::arch::asm!(
            "svc {immediate}",
            immediate = const dali_sdk::svc::GATEWAY_IMMEDIATE,
            inout("r0") status,
            options(nostack, preserves_flags),
        );
    }
    let _ = status;
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
