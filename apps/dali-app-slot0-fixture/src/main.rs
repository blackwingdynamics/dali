#![no_std]
#![no_main]

use core::panic::PanicInfo;

const TEST_MESSAGE: &str = "Slot 0 fixture";
const PROGRESS_STEP: u32 = 1;

#[used]
#[unsafe(link_section = ".data.slot0_fixture")]
static mut INITIALIZED_STATE: u32 = 0x2468_ACED;

#[used]
#[unsafe(link_section = ".bss.slot0_fixture")]
static mut ZERO_STATE: u32 = 0;

#[used]
#[unsafe(link_section = ".data.slot0_fixture")]
static mut PROGRESS_STATE: u32 = 0;

#[used]
#[unsafe(link_section = ".data.slot0_fixture")]
static FUNCTION_REFERENCE: unsafe extern "C" fn() -> ! = fixture_loop;

unsafe extern "C" fn fixture_loop() -> ! {
    loop {
        let progress = unsafe {
            // SAFETY: The fixture accesses only its own declared data region.
            core::ptr::read_volatile(core::ptr::addr_of!(PROGRESS_STATE))
        };
        unsafe {
            // SAFETY: The fixture writes only its own declared data region.
            core::ptr::write_volatile(
                core::ptr::addr_of_mut!(PROGRESS_STATE),
                progress.wrapping_add(PROGRESS_STEP),
            );
        }
    }
}

/// Exercises the slot 0 code, initialized-data, zero-data, and relocation path.
#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.amiran_entry")]
pub unsafe extern "C" fn amiran_entry() -> ! {
    let _ = dali_sdk::log(TEST_MESSAGE);
    let initialized = unsafe {
        // SAFETY: The fixture accesses only its own declared data region.
        core::ptr::read_volatile(core::ptr::addr_of!(INITIALIZED_STATE))
    };
    unsafe {
        // SAFETY: The fixture writes only its own declared data region.
        core::ptr::write_volatile(
            core::ptr::addr_of_mut!(INITIALIZED_STATE),
            initialized.wrapping_add(1),
        );
        // SAFETY: The fixture writes only its own declared zero-initialized data.
        core::ptr::write_volatile(core::ptr::addr_of_mut!(ZERO_STATE), initialized);
        // SAFETY: The stored function reference belongs to this fixture.
        let _ = core::ptr::read_volatile(core::ptr::addr_of!(FUNCTION_REFERENCE));
    }
    unsafe {
        // SAFETY: The stored target is this fixture's non-returning entry loop.
        fixture_loop()
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
