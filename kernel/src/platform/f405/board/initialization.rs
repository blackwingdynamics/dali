//! F405 singleton acquisition and board initialization.

#[cfg(feature = "driver-hardware-test")]
use super::super::drivers::F405DriverProbe;
use super::config::SdioPins;
use super::resources::{Board, TimerMode};
use dali_driver_api::{InterruptPin, InterruptTrigger};
use dali_targets::TARGET_F405;
use stm32f4xx_hal::{pac, prelude::*, time::Hertz};

#[cfg(feature = "usb-cdc")]
use super::UsbResources;

/// Takes singleton peripherals and initializes the STM32F405 board hardware.
pub fn initialize() -> Board {
    let device = pac::Peripherals::take().unwrap();
    let core = cortex_m::Peripherals::take().unwrap();
    let reset_cause = super::super::F405Watchdog::read_reset_cause(&device.RCC);
    super::super::F405Watchdog::clear_reset_cause(&device.RCC);
    let rcc = device.RCC.constrain();
    let clocks = rcc
        .cfgr
        .use_hse(Hertz::from_raw(TARGET_F405.clock.input_hz))
        .sysclk(Hertz::from_raw(super::SYSTEM_CLOCK_HZ))
        .hclk(Hertz::from_raw(super::SYSTEM_CLOCK_HZ))
        .pclk1(Hertz::from_raw(TARGET_F405.clock.pclk1_hz))
        .pclk2(Hertz::from_raw(TARGET_F405.clock.pclk2_hz));
    #[cfg(feature = "usb-cdc")]
    let clocks = clocks.require_pll48clk();
    let clocks = clocks.freeze();
    let delay = TimerMode::Delay(core.SYST.delay(&clocks));

    #[cfg(any(feature = "usb-cdc", feature = "driver-hardware-test"))]
    let gpioa = device.GPIOA.split();
    let gpiob = device.GPIOB.split();
    let gpioc = device.GPIOC.split();
    let gpiod = device.GPIOD.split();
    let status_led = super::super::drivers::F405GpioPin::new_output(gpiob.pb2.into_dynamic());
    let mut user_key = super::super::drivers::F405ExtiPin::new(
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
        super::super::unmask_exti15_10_irq();
    }

    #[cfg(feature = "driver-hardware-test")]
    let driver_probe = F405DriverProbe::new(
        device.USART1,
        device.SPI1,
        device.I2C1,
        (
            gpioa.pa9.into_alternate::<7>(),
            gpioa.pa10.into_alternate::<7>(),
        ),
        (
            gpioa.pa5.into_alternate::<5>(),
            gpioa.pa6.into_alternate::<5>(),
            gpioa.pa7.into_alternate::<5>(),
        ),
        (
            gpiob.pb6.into_alternate::<4>(),
            gpiob.pb7.into_alternate::<4>(),
        ),
        &clocks,
        TARGET_F405.driver_probe,
    );

    let sdio_pins: SdioPins = (
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
        #[cfg(feature = "driver-hardware-test")]
        driver_probe,
        sdio_pins: Some(sdio_pins),
        sdio: Some(device.SDIO),
        clocks,
        watchdog: Some(super::super::F405Watchdog::new(device.IWDG, reset_cause)),
        #[cfg(feature = "usb-cdc")]
        usb,
    }
}
