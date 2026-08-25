//! F405 board resource ownership and lifecycle state.

#[cfg(feature = "driver-hardware-test")]
use super::super::drivers::F405DriverProbe;
#[cfg(all(feature = "driver-hardware-test", not(feature = "display-oled")))]
use super::super::drivers::F405I2c;
#[cfg(feature = "display-oled")]
use super::config::OledDisplay;
use super::{SdioPins, StatusLed, UserKey};
use crate::runtime::watchdog::WatchdogBackend;
#[cfg(feature = "display-oled")]
use dali_driver_api::{DiagnosticsConsole, Duration};
use stm32f4xx_hal::{pac, rcc::Clocks, timer::SysDelay};

#[cfg(feature = "abi-context-switch")]
use super::super::drivers::F405TimerDriver;
#[cfg(feature = "usb-cdc")]
use super::UsbResources;

/// Peripherals owned by the kernel after board initialization.
pub struct Board {
    /// Blocking delay driven by the initialized SysTick timer.
    pub(super) delay: Option<TimerMode>,
    /// WeAct board status LED on active-high PB2.
    pub status_led: StatusLed,
    /// User key input reserved for board-local EXTI handling.
    pub user_key: UserKey,
    /// Whether the board-local EXTI line was enabled during initialization.
    pub(super) user_key_enabled: bool,
    #[cfg(feature = "driver-hardware-test")]
    pub(super) status_led_on: bool,
    #[cfg(feature = "driver-hardware-test")]
    pub(super) gpio_driver_log_emitted: bool,
    /// Test-only UART/SPI bounded-timeout resources.
    #[cfg(feature = "driver-hardware-test")]
    pub(super) driver_probe: Option<F405DriverProbe>,
    #[cfg(all(feature = "driver-hardware-test", not(feature = "display-oled")))]
    pub(super) i2c: Option<F405I2c>,
    #[cfg(feature = "display-oled")]
    pub(super) display: Option<OledDisplay>,
    #[cfg(feature = "display-oled")]
    pub(super) console:
        DiagnosticsConsole<{ super::config::DISPLAY_COLUMNS }, { super::config::DISPLAY_ROWS }>,
    /// Hardware SDIO 4-bit pins for the on-board microSD socket.
    pub(super) sdio_pins: Option<SdioPins>,
    /// SDIO peripheral reserved for the storage driver.
    pub(super) sdio: Option<pac::SDIO>,
    /// Frozen clock configuration required to initialize SDIO.
    pub(super) clocks: Clocks,
    /// Unarmed F405 independent watchdog backend.
    pub(super) watchdog: Option<super::super::F405Watchdog>,
    /// USB FS resources reserved for the CDC logging backend.
    #[cfg(feature = "usb-cdc")]
    pub(super) usb: Option<UsbResources>,
}

/// Exclusive ownership mode for the board's single SysTick peripheral.
pub(crate) enum TimerMode {
    /// Bootstrap and heartbeat delay mode.
    Delay(SysDelay),
    /// Scheduler interrupt mode after an application context is active.
    #[cfg(feature = "abi-context-switch")]
    Scheduler(F405TimerDriver),
}

impl Board {
    /// Queues and renders a bounded diagnostic line on the optional OLED.
    #[cfg(feature = "display-oled")]
    pub(crate) fn write_display_log(&mut self, bytes: &[u8]) {
        self.console.write(bytes);
        let Some(display) = self.display.as_mut() else {
            return;
        };
        let timeout = Duration::from_ticks(dali_targets::TARGET_F405.display.timeout_ticks);
        let _ = self.console.render(display, timeout);
    }

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
    pub fn take_watchdog(&mut self) -> Option<super::super::F405Watchdog> {
        self.watchdog.take()
    }
}
