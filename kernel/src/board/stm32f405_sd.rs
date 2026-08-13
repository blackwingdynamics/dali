//! WeAct Studio STM32F405RGT6 Core Board support.

use stm32f4xx_hal::{gpio, pac, prelude::*, rcc::Clocks, timer::SysDelay};

/// System clock target for the 8 MHz HSE board.
pub const SYSTEM_CLOCK_MHZ: u32 = 168;

/// Heartbeat interval used by the kernel bootstrap demonstration.
pub const HEARTBEAT_PERIOD_MS: u32 = 1_000;

/// Status LED output pin on the active-high PB2 LED.
pub type StatusLed = gpio::gpiob::PB2<gpio::Output<gpio::PushPull>>;

/// SDIO pins owned by the kernel after board initialization.
pub type SdioPins = (
    gpio::gpioc::PC12,
    gpio::gpiod::PD2,
    gpio::gpioc::PC8,
    gpio::gpioc::PC9,
    gpio::gpioc::PC10,
    gpio::gpioc::PC11,
);

/// Peripherals owned by the kernel after board initialization.
pub struct Board {
    /// Blocking delay driven by the initialized SysTick timer.
    pub delay: SysDelay,
    /// WeAct board status LED on active-high PB2.
    pub status_led: StatusLed,
    /// Hardware SDIO 4-bit pins for the on-board microSD socket.
    sdio_pins: Option<SdioPins>,
    /// SDIO peripheral reserved for the storage driver.
    sdio: Option<pac::SDIO>,
    /// Frozen clock configuration required to initialize SDIO.
    clocks: Clocks,
}

impl Board {
    /// Transfers the SDIO resources to the storage driver.
    pub fn take_sdio_resources(&mut self) -> Option<(pac::SDIO, SdioPins, &Clocks)> {
        let peripheral = self.sdio.take()?;
        let pins = self.sdio_pins.take()?;
        Some((peripheral, pins, &self.clocks))
    }
}

/// Takes singleton peripherals and initializes the STM32F405 board hardware.
pub fn initialize(device: pac::Peripherals, core: cortex_m::Peripherals) -> Board {
    let rcc = device.RCC.constrain();
    let clocks = rcc
        .cfgr
        .use_hse(8.MHz())
        .sysclk(SYSTEM_CLOCK_MHZ.MHz())
        .hclk(SYSTEM_CLOCK_MHZ.MHz())
        .pclk1(42.MHz())
        .pclk2(84.MHz())
        .freeze();
    let delay = core.SYST.delay(&clocks);

    let gpiob = device.GPIOB.split();
    let gpioc = device.GPIOC.split();
    let gpiod = device.GPIOD.split();
    let mut status_led = gpiob.pb2.into_push_pull_output();
    status_led.set_low();

    let sdio_pins = (
        gpioc.pc12,
        gpiod.pd2.internal_pull_up(true),
        gpioc.pc8.internal_pull_up(true),
        gpioc.pc9.internal_pull_up(true),
        gpioc.pc10.internal_pull_up(true),
        gpioc.pc11.internal_pull_up(true),
    );

    Board {
        delay,
        status_led,
        sdio_pins: Some(sdio_pins),
        sdio: Some(device.SDIO),
        clocks,
    }
}
