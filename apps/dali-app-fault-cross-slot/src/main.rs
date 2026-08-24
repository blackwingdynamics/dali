#![no_std]
#![no_main]

use core::panic::PanicInfo;

const TEST_MESSAGE: &str = "Fault injection: slot1 read of slot0 memory";
const TARGET_SLOT_ID: u8 = 0;
const TARGET_MEMORY: dali_targets::MemoryProfile = dali_targets::TARGET_F405.memory;

const fn target_slot_code_origin() -> usize {
    let Some(isolation) = TARGET_MEMORY.isolation else {
        return 0;
    };
    let mut index = 0;
    while index < isolation.slots.len() {
        if isolation.slots[index].id == TARGET_SLOT_ID {
            return isolation.slots[index].code_origin as usize;
        }
        index += 1;
    }
    0
}

const TARGET_SLOT_CODE_ORIGIN: usize = target_slot_code_origin();
const _: () = assert!(TARGET_SLOT_CODE_ORIGIN != 0);

/// Enters the slot-boundary fault test after proving SVC communication.
///
/// # Safety
///
/// The kernel invokes this symbol only after validating the AMRN package and
/// preparing the slot1 application PSP frame.
#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.amiran_entry")]
pub unsafe extern "C" fn amiran_entry() -> ! {
    let _ = dali_sdk::log(TEST_MESSAGE);
    let _value = unsafe {
        // SAFETY: This read intentionally targets the other manifest-owned
        // application slot to verify unprivileged MPU isolation.
        core::ptr::read_volatile(TARGET_SLOT_CODE_ORIGIN as *const u32)
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
