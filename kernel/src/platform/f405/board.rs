//! WeAct Studio STM32F405RGT6 Core Board hardware backend.

use super::drivers::{F405ExtiPin, F405GpioPin};
use crate::runtime::watchdog::WatchdogBackend;
use dali_driver_api::{InterruptPin, InterruptTrigger, OutputPin};
use dali_targets::TARGET_F405;

#[cfg(feature = "abi-context-switch")]
use super::drivers::F405TimerDriver;
#[cfg(feature = "abi-current")]
use dali_targets::MemoryProfile;
use stm32f4xx_hal::{gpio, pac, prelude::*, rcc::Clocks, time::Hertz, timer::SysDelay};

#[cfg(feature = "usb-cdc")]
mod usb;
#[cfg(feature = "usb-cdc")]
pub use usb::UsbResources;
#[cfg(feature = "usb-cdc")]
pub(crate) use usb::{pend_usb_irq, unmask_usb_irq};
mod input;
#[cfg(feature = "abi-context-switch")]
mod scheduler;
#[cfg(feature = "abi-context-switch")]
pub(crate) use scheduler::enable_scheduler_tick;

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
const _: () = assert!(TARGET_F405.amrn_target_id == dali_amrn::TARGET_ID);
#[cfg(feature = "abi-current")]
const _: () = assert!(crate::abi::CURRENT_VERSION == dali_amrn::v2::ABI_VERSION);
#[cfg(not(feature = "abi-current"))]
const _: () = assert!(TARGET_F405.abi_version == crate::abi::CURRENT_VERSION);
const _: () = assert!(
    TARGET_F405.memory.application_origin == dali_amrn::LOAD_ADDRESS
        && TARGET_F405.memory.application_length == dali_amrn::MAX_PAYLOAD_SIZE as u32
);

/// Port selected by the board manifest for the active-high status LED.
const STATUS_LED_PORT: char = 'B';
/// Pin selected by the board manifest for the active-high status LED.
const STATUS_LED_PIN: u8 = 2;

/// Status LED output pin on the active-high PB2 LED.
pub type StatusLed = F405GpioPin<STATUS_LED_PORT, STATUS_LED_PIN>;

/// Port selected by the board manifest for the active-low user key.
const USER_KEY_PORT: char = 'C';
/// Pin selected by the board manifest for the active-low user key.
const USER_KEY_PIN: u8 = 13;

/// Board user-key input bound to the PC13 EXTI line.
pub type UserKey = F405ExtiPin<USER_KEY_PORT, USER_KEY_PIN>;

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
    /// User key input reserved for board-local EXTI handling.
    pub user_key: UserKey,
    /// Whether the board-local EXTI line was enabled during initialization.
    user_key_enabled: bool,
    #[cfg(feature = "driver-hardware-test")]
    status_led_on: bool,
    #[cfg(feature = "driver-hardware-test")]
    gpio_driver_log_emitted: bool,
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
    Scheduler(F405TimerDriver),
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

    /// Transfers the unarmed watchdog backend to the kernel runtime owner.
    pub fn take_watchdog(&mut self) -> Option<super::F405Watchdog> {
        self.watchdog.take()
    }
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

    // GPIOA is split only for USB FS PA11/PA12. PA13/PA14 remain untouched
    // so the SWD debug connection stays in its reset-time AF0 configuration.
    #[cfg(feature = "usb-cdc")]
    let gpioa = device.GPIOA.split();
    let gpiob = device.GPIOB.split();
    let gpioc = device.GPIOC.split();
    let gpiod = device.GPIOD.split();
    let status_led = F405GpioPin::new_output(gpiob.pb2.into_dynamic());
    let mut user_key = F405ExtiPin::new(
        gpioc.pc13.into_pull_up_input(),
        device.EXTI,
        device.SYSCFG.constrain(),
    );
    let user_key_enabled = matches!(
        user_key.enable_interrupt(InterruptTrigger::FallingEdge, None),
        Ok(())
    );
    #[cfg(feature = "driver-hardware-test")]
    if user_key_enabled {
        super::unmask_exti15_10_irq();
    }

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
        user_key,
        user_key_enabled,
        #[cfg(feature = "driver-hardware-test")]
        status_led_on: false,
        #[cfg(feature = "driver-hardware-test")]
        gpio_driver_log_emitted: false,
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
    let result = if on {
        OutputPin::set_high(&mut board.status_led)
    } else {
        OutputPin::set_low(&mut board.status_led)
    };
    if result.is_err() {
        crate::logging::error(
            crate::logging::BOOT_SUBSYSTEM,
            format_args!("[GPIO] Status LED operation failed"),
        );
    }
    #[cfg(feature = "driver-hardware-test")]
    if result.is_ok() && board.status_led_on != on {
        board.status_led_on = on;
        if !board.gpio_driver_log_emitted {
            board.gpio_driver_log_emitted = true;
            crate::logging::info(
                crate::logging::BOOT_SUBSYSTEM,
                format_args!("[DRIVER][GPIO] Hardware pin toggled"),
            );
        }
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
