//! F405 bounded-I/O acceptance probe.

use super::{F405I2c, F405Spi, F405TimeoutConfig, F405Uart};
use dali_driver_api::{
    BaudRate, DataBits, Duration, I2cAddress, I2cDriver, Parity, SerialConfig, SerialConfigure,
    SerialOwnership, SerialRead, SpiBusOwnership, SpiDeviceId, SpiDeviceSelect, SpiTransfer,
    StopBits,
};
use dali_targets::{DriverProbeProfile, TARGET_F405};
use stm32f4xx_hal::{
    gpio::{self, Alternate},
    pac,
    prelude::*,
    rcc::Clocks,
    serial::{Config as HalSerialConfig, SerialExt},
    spi::{Mode, Phase, Polarity, SpiExt},
    time::Hertz,
};

/// Result of the real peripheral timeout probes.
pub(crate) struct F405DriverProbeResult {
    /// Whether the UART receive path returned its bounded timeout.
    pub(crate) uart_timed_out: bool,
    /// Whether the SPI receive path returned its bounded timeout.
    pub(crate) spi_timed_out: bool,
    /// Whether the SPI transfer completed after the peripheral was restored.
    pub(crate) spi_recovered: bool,
    /// Whether the I2C probe reached a bounded terminal result.
    pub(crate) i2c_completed: bool,
    /// Whether the I2C backend recovered its peripheral after a bus fault.
    pub(crate) i2c_recovered: bool,
}

/// F405 UART and SPI adapters retained for the acceptance probe.
pub(crate) struct F405DriverProbe {
    uart: F405Uart,
    spi: F405Spi<pac::SPI1>,
    i2c: F405I2c,
    uart_config: SerialConfig,
    probe_config: DriverProbeProfile,
}

impl F405DriverProbe {
    /// Initializes test-only UART1 and SPI1 resources on documented free pins.
    pub(crate) fn new(
        usart: pac::USART1,
        spi: pac::SPI1,
        i2c: pac::I2C1,
        uart_pins: (
            gpio::gpioa::PA9<Alternate<7>>,
            gpio::gpioa::PA10<Alternate<7>>,
        ),
        spi_pins: (
            gpio::gpioa::PA5<Alternate<5>>,
            gpio::gpioa::PA6<Alternate<5>>,
            gpio::gpioa::PA7<Alternate<5>>,
        ),
        i2c_pins: (
            gpio::gpiob::PB6<Alternate<4>>,
            gpio::gpiob::PB7<Alternate<4>>,
        ),
        clocks: &Clocks,
        probe_config: DriverProbeProfile,
    ) -> Option<Self> {
        let baud_rate = BaudRate::from_bits_per_second(probe_config.uart_baud_rate_hz).ok()?;
        let uart_config =
            SerialConfig::new(baud_rate, DataBits::Eight, Parity::None, StopBits::One);
        let serial = match usart.serial(
            uart_pins,
            HalSerialConfig::default()
                .baudrate(probe_config.uart_baud_rate_hz.bps())
                .wordlength_8()
                .parity_none(),
            clocks,
        ) {
            Ok(serial) => serial,
            Err(_) => return None,
        };
        let spi = spi.spi(
            spi_pins,
            Mode {
                polarity: Polarity::IdleLow,
                phase: Phase::CaptureOnFirstTransition,
            },
            Hertz::from_raw(probe_config.spi_clock_hz),
            clocks,
        );
        let timeout = F405TimeoutConfig::new(
            Duration::from_ticks(probe_config.timeout_ticks),
            probe_config.polls_per_timeout_tick,
        );
        let i2c = F405I2c::new(
            i2c,
            i2c_pins,
            clocks,
            timeout,
            TARGET_F405.i2c.bus_frequency_hz,
        )?;
        Some(Self {
            uart: F405Uart::new(serial, timeout, uart_config),
            spi: F405Spi::new(spi, timeout),
            i2c,
            uart_config,
            probe_config,
        })
    }

    /// Runs one bounded no-peer receive test on each initialized peripheral.
    pub(crate) fn run(&mut self) -> F405DriverProbeResult {
        let mut uart_byte = [0; TARGET_F405.driver_probe.buffer_length];
        let uart_timed_out = self.run_uart_probe(&mut uart_byte);
        let mut spi_byte =
            [self.probe_config.spi_fill_byte; TARGET_F405.driver_probe.buffer_length];
        let (spi_timed_out, spi_recovered) = self.run_spi_probe(&mut spi_byte);
        let (i2c_completed, i2c_recovered) = self.run_i2c_probe();
        F405DriverProbeResult {
            uart_timed_out,
            spi_timed_out,
            spi_recovered,
            i2c_completed,
            i2c_recovered,
        }
    }

    /// Runs the owned UART timeout probe and always releases the UART.
    fn run_uart_probe(&mut self, buffer: &mut [u8]) -> bool {
        if self.uart.acquire().is_err() {
            return false;
        }
        let timed_out = self.uart.configure(self.uart_config).is_ok()
            && matches!(
                self.uart.read(buffer, self.probe_timeout()),
                Err(dali_driver_api::DriverError::Timeout)
            );
        timed_out && self.uart.release().is_ok()
    }

    /// Runs the logically selected SPI timeout probe and balances its state.
    fn run_spi_probe(&mut self, buffer: &mut [u8]) -> (bool, bool) {
        if self.spi.acquire().is_err() {
            return (false, false);
        }
        let selected = self
            .spi
            .select(SpiDeviceId::new(self.probe_config.spi_device_id))
            .is_ok();
        if !selected {
            let _ = self.spi.release();
            return (false, false);
        }
        self.spi.disable_for_probe();
        let timed_out = matches!(
            self.spi.transfer(buffer, self.probe_timeout()),
            Err(dali_driver_api::DriverError::Timeout)
        );
        self.spi.enable_for_probe();
        let recovered = timed_out && self.spi.transfer(buffer, self.probe_timeout()).is_ok();
        let deselected = selected && self.spi.deselect().is_ok();
        let released = deselected && self.spi.release().is_ok();
        (timed_out && released, recovered && released)
    }

    /// Runs a bounded I2C write-read transaction and records recovery state.
    fn run_i2c_probe(&mut self) -> (bool, bool) {
        let mut response = [0; TARGET_F405.driver_probe.buffer_length];
        if self.i2c.acquire().is_err() {
            return (false, false);
        }
        let result = self.i2c.write_read(
            I2cAddress::new(self.probe_config.i2c_address),
            &[self.probe_config.spi_fill_byte],
            &mut response,
            self.probe_timeout(),
        );
        let recovered = self.i2c.recovery_observed();
        let completed = result.is_ok() || matches!(result, Err(dali_driver_api::DriverError::Nack));
        let _ = self.i2c.release();
        (completed, recovered)
    }

    fn probe_timeout(&self) -> Duration {
        Duration::from_ticks(self.probe_config.timeout_ticks)
    }
}
