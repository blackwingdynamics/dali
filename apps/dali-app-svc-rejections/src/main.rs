#![no_std]
#![no_main]

use core::panic::PanicInfo;

const TARGET_MEMORY: dali_targets::MemoryProfile = dali_targets::TARGET_F405.memory;
const UNKNOWN_SERVICE: u32 = u32::MAX;
const LOG_SERVICE: u32 = dali_sdk::svc::ServiceId::Log as u32;
const STATUS_REJECTED: u32 = dali_sdk::svc::STATUS_REJECTED;
static INVALID_UTF8: [u8; 1] = [0xFF];

const fn kernel_probe_address() -> usize {
    TARGET_MEMORY.kernel_origin as usize
}

const fn peripheral_probe_address() -> usize {
    match TARGET_MEMORY.isolation {
        Some(isolation) => match isolation.peripheral_origin {
            Some(origin) => origin as usize,
            None => 0,
        },
        None => 0,
    }
}

const KERNEL_PROBE_ADDRESS: usize = kernel_probe_address();
const PERIPHERAL_PROBE_ADDRESS: usize = peripheral_probe_address();
const _: () = assert!(PERIPHERAL_PROBE_ADDRESS != 0);

/// Runs the bounded SVC rejection matrix and then remains alive.
///
/// # Safety
///
/// The kernel invokes this symbol only after validating the ABI v3 package and
/// preparing the application PSP frame.
#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.amiran_entry")]
pub unsafe extern "C" fn amiran_entry() -> ! {
    let valid_message = "SVC rejection matrix passed";
    let oversized_length = dali_sdk::MAX_LOG_MESSAGE_BYTES + 1;

    report_rejection(
        "SVC rejected unknown service",
        unsafe { raw_svc(UNKNOWN_SERVICE, valid_message.as_ptr(), valid_message.len()) },
    );
    report_rejection(
        "SVC rejected kernel pointer",
        unsafe { raw_svc(LOG_SERVICE, KERNEL_PROBE_ADDRESS as *const u8, 1) },
    );
    report_rejection(
        "SVC rejected peripheral pointer",
        unsafe { raw_svc(LOG_SERVICE, PERIPHERAL_PROBE_ADDRESS as *const u8, 1) },
    );
    report_rejection(
        "SVC rejected oversized message",
        unsafe { raw_svc(LOG_SERVICE, valid_message.as_ptr(), oversized_length) },
    );
    report_rejection(
        "SVC rejected invalid UTF-8",
        unsafe { raw_svc(LOG_SERVICE, INVALID_UTF8.as_ptr(), INVALID_UTF8.len()) },
    );
    let _ = dali_sdk::log("SVC rejection matrix complete");
    loop {
        core::hint::spin_loop();
    }
}

fn report_rejection(message: &str, status: u32) {
    if status == STATUS_REJECTED {
        let _ = dali_sdk::log(message);
    }
}

unsafe fn raw_svc(service: u32, pointer: *const u8, length: usize) -> u32 {
    let mut status = service;
    let pointer = pointer as usize;
    unsafe {
        // SAFETY: This fixture intentionally sends bounded and malformed
        // values through the ABI v3 gateway to verify kernel rejection.
        core::arch::asm!(
            "svc {immediate}",
            immediate = const dali_sdk::svc::GATEWAY_IMMEDIATE,
            inout("r0") status,
            in("r1") pointer,
            in("r2") length,
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
