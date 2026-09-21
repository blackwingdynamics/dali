#![no_std]
#![no_main]

use core::panic::PanicInfo;

const TEST_DMA_REQUEST_SERVICE: u32 = dali_sdk::svc::TEST_DMA_REQUEST_SERVICE;
const STATUS_REJECTED: u32 = dali_sdk::svc::STATUS_REJECTED;
const ACCEPTANCE_MESSAGE: &str = "DMA application request rejected";

/// Requests the kernel DMA service and reports the required denial result.
///
/// # Safety
///
/// The kernel invokes this symbol only after validating the ABI v4 cartridge and
/// preparing the application PSP frame.
#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.amiran_entry")]
pub unsafe extern "C" fn amiran_entry() -> ! {
    let mut status = TEST_DMA_REQUEST_SERVICE;
    unsafe {
        // SAFETY: This non-production fixture sends only the test service ID
        // through the bounded ABI gateway; it supplies no DMA pointer.
        core::arch::asm!(
            "svc {immediate}",
            immediate = const dali_sdk::svc::GATEWAY_IMMEDIATE,
            inout("r0") status,
            options(nostack, preserves_flags),
        );
    }
    if status == STATUS_REJECTED {
        let _ = dali_sdk::log(ACCEPTANCE_MESSAGE);
    }
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
