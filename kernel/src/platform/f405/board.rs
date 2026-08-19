//! WeAct Studio STM32F405RGT6 Core Board hardware backend.

use crate::runtime::watchdog::WatchdogBackend;
use dali_targets::TARGET_F405;

#[cfg(feature = "abi-current")]
use dali_targets::MemoryProfile;
#[cfg(feature = "abi-context-switch")]
use stm32f4xx_hal::timer::{SysCounterHz, SysEvent};
use stm32f4xx_hal::{gpio, pac, prelude::*, rcc::Clocks, time::Hertz, timer::SysDelay};

/// First planned single-application F405 isolation layout.
pub const ISOLATION_LAYOUT: Option<crate::security::mpu::IsolationLayout> =
    crate::security::mpu::IsolationLayout::from_memory(TARGET_F405.memory);
const _: () = assert!(ISOLATION_LAYOUT.is_some());

/// Activates unprivileged application permissions after the loader finishes.
#[cfg(feature = "abi-mpu")]
pub fn activate_application_regions(slot: dali_targets::IsolationSlot) -> bool {
    let Some(layout) =
        crate::security::mpu::IsolationLayout::from_memory_for_slot(TARGET_F405.memory, slot)
    else {
        return false;
    };
    crate::security::mpu::activate_application_regions(layout);
    true
}

/// The current MVP board supports AMRN native application execution.
#[cfg(not(feature = "abi-current"))]
pub const APPLICATION_EXECUTION_SUPPORTED: bool = true;

/// System clock target derived from the declarative F405 target profile.
pub const SYSTEM_CLOCK_HZ: u32 = TARGET_F405.clock.system_hz;
#[cfg(feature = "abi-current")]
pub const MEMORY_PROFILE: MemoryProfile = TARGET_F405.memory;
/// Unit conversion used by the boot log's human-readable clock value.
const HZ_PER_MHZ: u32 = 1_000_000;
/// System clock in megahertz for the common platform facade.
pub const SYSTEM_CLOCK_MHZ: u32 = SYSTEM_CLOCK_HZ / HZ_PER_MHZ;
#[cfg(feature = "abi-context-switch")]
const SYSTICK_MIN_RELOAD: u32 = 1;
#[cfg(feature = "abi-context-switch")]
const SYSTICK_MAX_RELOAD: u32 = 0x00FF_FFFF;

const _: () = assert!(TARGET_F405.amrn_target_id == dali_amrn::TARGET_ID);
#[cfg(feature = "abi-current")]
const _: () = assert!(crate::abi::CURRENT_VERSION == dali_amrn::v2::ABI_VERSION);
#[cfg(not(feature = "abi-current"))]
const _: () = assert!(TARGET_F405.abi_version == crate::abi::CURRENT_VERSION);
const _: () = assert!(
    TARGET_F405.memory.application_origin == dali_amrn::LOAD_ADDRESS
        && TARGET_F405.memory.application_length == dali_amrn::MAX_PAYLOAD_SIZE as u32
);

/// Status LED output pin on the active-high PB2 LED.
pub type StatusLed = gpio::gpiob::PB2<gpio::Output<gpio::PushPull>>;

/// USB FS resources connected to the board's USB-C data pins.
#[cfg(feature = "usb-cdc")]
pub struct UsbResources {
    /// USB global registers.
    pub global: pac::OTG_FS_GLOBAL,
    /// USB device registers.
    pub device: pac::OTG_FS_DEVICE,
    /// USB power and clock registers.
    pub power_clock: pac::OTG_FS_PWRCLK,
    /// USB D- pin on PA11.
    pub dm: gpio::gpioa::PA11<gpio::Alternate<10>>,
    /// USB D+ pin on PA12.
    pub dp: gpio::gpioa::PA12<gpio::Alternate<10>>,
    /// Frozen clocks used to configure the USB peripheral.
    pub clocks: Clocks,
}

#[cfg(feature = "usb-cdc")]
impl crate::logging::usb_cdc::UsbResources for UsbResources {
    type Bus = stm32f4xx_hal::otg_fs::UsbBusType;

    fn into_bus(
        self,
        endpoint_memory: &'static mut [u32],
    ) -> usb_device::bus::UsbBusAllocator<Self::Bus> {
        let usb = stm32f4xx_hal::otg_fs::USB::new(
            (self.global, self.device, self.power_clock),
            (self.dm, self.dp),
            &self.clocks,
        );
        stm32f4xx_hal::otg_fs::UsbBus::new(usb, endpoint_memory)
    }
}

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
    delay: Option<TimerMode>,
    /// WeAct board status LED on active-high PB2.
    pub status_led: StatusLed,
    /// Hardware SDIO 4-bit pins for the on-board microSD socket.
    sdio_pins: Option<SdioPins>,
    /// SDIO peripheral reserved for the storage driver.
    sdio: Option<pac::SDIO>,
    /// Frozen clock configuration required to initialize SDIO.
    clocks: Clocks,
    /// Unarmed F405 independent watchdog backend.
    watchdog: Option<super::F405Watchdog>,
    /// USB FS resources reserved for the CDC logging backend.
    #[cfg(feature = "usb-cdc")]
    usb: Option<UsbResources>,
}

/// Exclusive ownership mode for the board's single SysTick peripheral.
enum TimerMode {
    /// Bootstrap and heartbeat delay mode.
    Delay(SysDelay),
    /// Scheduler interrupt mode after an application context is active.
    #[cfg(feature = "abi-context-switch")]
    Scheduler(SysCounterHz),
}

impl Board {
    /// Transfers the SDIO resources to the storage driver.
    pub fn take_sdio_resources(&mut self) -> Option<(pac::SDIO, SdioPins, &Clocks)> {
        let peripheral = self.sdio.take()?;
        let pins = self.sdio_pins.take()?;
        Some((peripheral, pins, &self.clocks))
    }

    /// Transfers USB FS resources to the logging backend.
    #[cfg(feature = "usb-cdc")]
    pub fn take_usb_resources(&mut self) -> Option<UsbResources> {
        self.usb.take()
    }

    /// Returns the reset source captured during early bootstrap.
    pub fn reset_cause(&self) -> crate::runtime::watchdog::ResetCause {
        self.watchdog
            .as_ref()
            .map_or(crate::runtime::watchdog::ResetCause::Unknown, |watchdog| {
                watchdog.reset_cause()
            })
    }
}

/// Enables SysTick for the scheduler after the application context is ready.
#[cfg(feature = "abi-context-switch")]
pub fn enable_scheduler_tick(board: &mut Board, tick_hz: u32) -> bool {
    let Some(core_ticks) = SYSTEM_CLOCK_HZ.checked_div(tick_hz) else {
        return false;
    };
    let Some(reload) = core_ticks.checked_sub(1) else {
        return false;
    };
    if !(SYSTICK_MIN_RELOAD..=SYSTICK_MAX_RELOAD).contains(&reload) {
        return false;
    }

    let Some(TimerMode::Delay(delay)) = board.delay.take() else {
        return false;
    };
    let mut counter = delay.release().counter_hz();
    if counter.start(Hertz::from_raw(tick_hz)).is_err() {
        return false;
    }
    counter.listen(SysEvent::Update);
    board.delay = Some(TimerMode::Scheduler(counter));
    true
}

/// Takes singleton peripherals and initializes the STM32F405 board hardware.
pub fn initialize() -> Board {
    // The platform backend owns singleton acquisition so the kernel core does
    // not depend on the STM32 PAC or the reset-time peripheral topology.
    let device = pac::Peripherals::take().unwrap();
    let core = cortex_m::Peripherals::take().unwrap();
    let reset_cause = super::F405Watchdog::read_reset_cause(&device.RCC);
    super::F405Watchdog::clear_reset_cause(&device.RCC);
    let rcc = device.RCC.constrain();
    let clocks = rcc
        .cfgr
        .use_hse(Hertz::from_raw(TARGET_F405.clock.input_hz))
        .sysclk(Hertz::from_raw(TARGET_F405.clock.system_hz))
        .hclk(Hertz::from_raw(TARGET_F405.clock.system_hz))
        .pclk1(Hertz::from_raw(TARGET_F405.clock.pclk1_hz))
        .pclk2(Hertz::from_raw(TARGET_F405.clock.pclk2_hz));
    #[cfg(feature = "usb-cdc")]
    let clocks = clocks.require_pll48clk();
    let clocks = clocks.freeze();
    let delay = TimerMode::Delay(core.SYST.delay(&clocks));

    let gpiob = device.GPIOB.split();
    #[cfg(feature = "usb-cdc")]
    let gpioa = device.GPIOA.split();
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

    #[cfg(feature = "usb-cdc")]
    let usb = Some(UsbResources {
        global: device.OTG_FS_GLOBAL,
        device: device.OTG_FS_DEVICE,
        power_clock: device.OTG_FS_PWRCLK,
        dm: gpioa.pa11.into_alternate::<10>(),
        dp: gpioa.pa12.into_alternate::<10>(),
        clocks,
    });

    Board {
        delay: Some(delay),
        status_led,
        sdio_pins: Some(sdio_pins),
        sdio: Some(device.SDIO),
        clocks,
        watchdog: Some(super::F405Watchdog::new(device.IWDG, reset_cause)),
        #[cfg(feature = "usb-cdc")]
        usb,
    }
}

/// Sets the active-high F405 board LED to the requested logical state.
pub fn set_status_led(board: &mut Board, on: bool) {
    if on {
        board.status_led.set_high();
    } else {
        board.status_led.set_low();
    }
}

/// Delays through the bootstrap-owned SysTick mode.
pub fn delay_ms(board: &mut Board, milliseconds: u32) {
    let Some(mode) = board.delay.take() else {
        return;
    };
    let delay = match mode {
        TimerMode::Delay(mut delay) => {
            delay.delay_ms(milliseconds);
            delay
        }
        #[cfg(feature = "abi-context-switch")]
        TimerMode::Scheduler(counter) => {
            // The scheduler mode is installed immediately before the
            // non-returning application launch. Keeping it installed preserves
            // SysTick ownership if an unexpected caller reaches the heartbeat.
            board.delay = Some(TimerMode::Scheduler(counter));
            return;
        }
    };
    board.delay = Some(TimerMode::Delay(delay));
}

/// Enables the board's USB interrupt after the CDC backend is initialized.
#[cfg(feature = "usb-cdc")]
pub fn unmask_usb_irq() {
    // SAFETY: The backend initializes USB state before unmasking its sole IRQ.
    unsafe { cortex_m::peripheral::NVIC::unmask(pac::Interrupt::OTG_FS) };
}

/// Wakes the board's USB backend after a main-context log enqueue.
#[cfg(feature = "usb-cdc")]
pub fn pend_usb_irq() {
    // SAFETY: PENDING is a software wake-up for the initialized USB owner.
    cortex_m::peripheral::NVIC::pend(pac::Interrupt::OTG_FS);
}
