#![no_std]
#![no_main]

use core::panic::PanicInfo;

const GPIOB_BASE_ADDRESS: usize = 0x4002_0400;
const GPIOB_MODER_OFFSET: usize = 0x00;
const GPIOB_ODR_OFFSET: usize = 0x14;
const PB2_PIN: u32 = 2;
const GPIO_MODE_BITS: u32 = 2;
const GPIO_OUTPUT_MODE: u32 = 0b01;
const LED_HALF_PERIOD_ITERATIONS: u32 = 1_000_000;

/// Native MVP entry point linked at the contract load address.
#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.amiran_entry")]
pub unsafe extern "C" fn amiran_entry() -> ! {
    configure_status_led();
    loop {
        set_status_led(true);
        delay_half_period();
        set_status_led(false);
        delay_half_period();
    }
}

fn configure_status_led() {
    let moder_address = GPIOB_BASE_ADDRESS + GPIOB_MODER_OFFSET;
    let pin_shift = PB2_PIN * GPIO_MODE_BITS;
    let pin_mask = GPIO_OUTPUT_MODE << pin_shift;
    let mode_mask = GPIO_OUTPUT_MODE << pin_shift;
    let moder = unsafe {
        // SAFETY: The F405 board contract assigns GPIOB MODER to this documented MMIO address.
        core::ptr::read_volatile(moder_address as *const u32)
    };
    unsafe {
        // SAFETY: Only the documented PB2 mode bits are changed; all other GPIOB modes remain.
        core::ptr::write_volatile(moder_address as *mut u32, (moder & !mode_mask) | pin_mask);
    }
}

fn set_status_led(on: bool) {
    let odr_address = (GPIOB_BASE_ADDRESS + GPIOB_ODR_OFFSET) as *mut u32;
    let pin_mask = 1_u32 << PB2_PIN;
    let odr = unsafe {
        // SAFETY: The F405 board contract assigns GPIOB ODR to this documented MMIO address.
        core::ptr::read_volatile(odr_address)
    };
    let next = if on { odr | pin_mask } else { odr & !pin_mask };
    unsafe {
        // SAFETY: Only the documented PB2 output bit is changed.
        core::ptr::write_volatile(odr_address, next);
    }
}

fn delay_half_period() {
    for _ in 0..LED_HALF_PERIOD_ITERATIONS {
        core::hint::spin_loop();
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
