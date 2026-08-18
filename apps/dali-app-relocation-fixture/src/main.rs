#![no_std]
#![no_main]

use core::panic::PanicInfo;

const TEST_MESSAGE: &str = "Relocation fixture";
const PROGRESS_STEP: u32 = 1;

#[used]
#[unsafe(link_section = ".data.relocation_fixture")]
static mut INITIALIZED_STATE: u32 = 0x1234_5678;

#[used]
#[unsafe(link_section = ".bss.relocation_fixture")]
static mut ZERO_STATE: u32 = 0;

#[used]
#[unsafe(link_section = ".data.relocation_fixture")]
static mut PROGRESS_STATE: u32 = 0;

#[used]
#[unsafe(link_section = ".data.relocation_fixture")]
static FUNCTION_REFERENCE: unsafe extern "C" fn() -> ! = relocation_target;

unsafe extern "C" fn relocation_target() -> ! {
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

/// Exercises code references, initialized data, zero data, and a data-held
/// function address for the relocation artifact fixture.
///
/// # Safety
///
/// The fixture is entered only by a test loader after its package contract is
/// validated.
#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.amiran_entry")]
pub unsafe extern "C" fn amiran_entry() -> ! {
    let _ = dali::log(TEST_MESSAGE);

    let initialized = unsafe {
        // SAFETY: The fixture deliberately accesses its own declared data.
        core::ptr::read_volatile(core::ptr::addr_of!(INITIALIZED_STATE))
    };
    unsafe {
        // SAFETY: The fixture deliberately accesses its own declared data.
        core::ptr::write_volatile(
            core::ptr::addr_of_mut!(INITIALIZED_STATE),
            initialized.wrapping_add(1),
        );
        // SAFETY: The fixture deliberately accesses its own declared zero data.
        core::ptr::write_volatile(core::ptr::addr_of_mut!(ZERO_STATE), initialized);
        // SAFETY: Reading the fixture's own function reference is intentional.
        let _ = core::ptr::read_volatile(core::ptr::addr_of!(FUNCTION_REFERENCE));
    }

    unsafe {
        // SAFETY: The fixture's function reference points to its own
        // non-returning relocation target.
        relocation_target()
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
