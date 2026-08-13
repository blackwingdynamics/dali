#![no_std]
#![no_main]

use cortex_m_rt::entry;
use panic_halt as _;
use rtt_target::{rprintln, rtt_init_print};
use stm32f4xx_hal::{pac, prelude::*};

#[entry]
fn main() -> ! {
    // Initialize Segger RTT channel for host logging.
    rtt_init_print!();
    rprintln!("====================================");
    rprintln!("   Dali OS Kernel Booting...       ");
    rprintln!("====================================");

    // Take ownership of core and MCU peripherals.
    let dp = pac::Peripherals::take().unwrap();
    let cp = cortex_m::Peripherals::take().unwrap();

    // Configure the STM32F411 system clock at the documented 100 MHz target.
    let rcc = dp.RCC.constrain();
    let clocks = rcc.cfgr.sysclk(100.MHz()).freeze();

    rprintln!("[BOOT] System clock: 100 MHz");

    // Initialize the blocking SysTick delay provider.
    let mut delay = cp.SYST.delay(&clocks);

    // Configure the WeAct BlackPill status LED on PC13.
    let gpioc = dp.GPIOC.split();
    let mut led = gpioc.pc13.into_push_pull_output();

    rprintln!("[BOOT] Hardware bootstrap complete");
    rprintln!("[BOOT] Entering kernel heartbeat");

    let mut heartbeat_counter: u64 = 0;

    loop {
        led.toggle();
        heartbeat_counter = heartbeat_counter.wrapping_add(1);

        rprintln!("[BOOT] Heartbeat: {}", heartbeat_counter);

        delay.delay_ms(1000_u32);
    }
}
