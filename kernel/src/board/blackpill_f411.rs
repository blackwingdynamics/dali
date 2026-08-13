//! WeAct BlackPill STM32F411 board support.

use stm32f4xx_hal::{gpio, pac, prelude::*, timer::SysDelay};

/// System clock target in megahertz for the STM32F411 MVP board.
pub const SYSTEM_CLOCK_MHZ: u32 = 100;

/// Status LED output pin after board initialization.
pub type StatusLed = gpio::gpioc::PC13<gpio::Output<gpio::PushPull>>;

/// Peripherals owned by the kernel after board initialization.
pub struct Board {
    /// Blocking delay driven by the initialized SysTick timer.
    pub delay: SysDelay,
    /// WeAct BlackPill status LED on PC13.
    pub status_led: StatusLed,
}

/// Takes singleton peripherals and initializes the BlackPill MVP hardware.
pub fn initialize(device: pac::Peripherals, core: cortex_m::Peripherals) -> Board {
    let rcc = device.RCC.constrain();
    let clocks = rcc.cfgr.sysclk(SYSTEM_CLOCK_MHZ.MHz()).freeze();
    let delay = core.SYST.delay(&clocks);
    let status_led = device.GPIOC.split().pc13.into_push_pull_output();

    Board { delay, status_led }
}

/// Sets the active-low BlackPill LED to the requested logical state.
pub fn set_status_led(board: &mut Board, on: bool) {
    if on {
        board.status_led.set_low();
    } else {
        board.status_led.set_high();
    }
}
