//! F405 bounded-I/O acceptance probe.

use super::{F405Spi, F405TimeoutConfig, F405Uart};
use dali_driver_api::{
    BaudRate, DataBits, Duration, Parity, SerialConfig, SerialConfigure, SerialOwnership,
    SerialRead, SpiBusOwnership, SpiDeviceId, SpiDeviceSelect, SpiTransfer, StopBits,
};
use stm32f4xx_hal::{
    gpio::{self, Alternate},
    pac,
    prelude::*,
    rcc::Clocks,
    serial::{Config as HalSerialConfig, SerialExt},
    spi::{Mode, Phase, Polarity, SpiExt},
    time::Hertz,
};

/// UART baud rate used by the opt-in F405 driver acceptance probe.
const PROBE_UART_BAUD_HZ: u32 = 115_200;
/// SPI clock used by the opt-in F405 driver acceptance probe.
const PROBE_SPI_CLOCK_HZ: u32 = 1_000_000;
/// Maximum timeout accepted by the acceptance probe adapters.
const PROBE_MAX_TIMEOUT_TICKS: u32 = 100;
/// Number of hardware polls represented by one contract timeout tick.
const PROBE_POLLS_PER_TIMEOUT_TICK: u32 = 1;
/// Timeout used by each no-peer bounded-I/O probe.
const PROBE_TIMEOUT: Duration = Duration::from_ticks(PROBE_MAX_TIMEOUT_TICKS);
/// Number of bytes used by each no-peer receive probe.
const PROBE_BUFFER_LENGTH: usize = 1;
/// Initial byte sent by the SPI timeout probe.
const PROBE_SPI_FILL_BYTE: u8 = 0xff;
/// Logical device selector reserved by this no-peer SPI acceptance probe.
const PROBE_SPI_DEVICE_ID: u8 = 0;

/// Result of the real peripheral timeout probes.
pub(crate) struct F405DriverProbeResult {
    /// Whether the UART receive path returned its bounded timeout.
    pub(crate) uart_timed_out: bool,
    /// Whether the SPI receive path returned its bounded timeout.
    pub(crate) spi_timed_out: bool,
}

/// F405 UART and SPI adapters retained for the acceptance probe.
pub(crate) struct F405DriverProbe {
    uart: F405Uart,
    spi: F405Spi<pac::SPI1>,
    uart_config: SerialConfig,
}

impl F405DriverProbe {
    /// Initializes test-only UART1 and SPI1 resources on documented free pins.
    pub(crate) fn new(
        usart: pac::USART1,
        spi: pac::SPI1,
        uart_pins: (
            gpio::gpioa::PA9<Alternate<7>>,
            gpio::gpioa::PA10<Alternate<7>>,
        ),
        spi_pins: (
            gpio::gpioa::PA5<Alternate<5>>,
            gpio::gpioa::PA6<Alternate<5>>,
            gpio::gpioa::PA7<Alternate<5>>,
        ),
        clocks: &Clocks,
    ) -> Option<Self> {
        let baud_rate = BaudRate::from_bits_per_second(PROBE_UART_BAUD_HZ).ok()?;
        let uart_config =
            SerialConfig::new(baud_rate, DataBits::Eight, Parity::None, StopBits::One);
        let serial = match usart.serial(
            uart_pins,
            HalSerialConfig::default()
                .baudrate(PROBE_UART_BAUD_HZ.bps())
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
            Hertz::from_raw(PROBE_SPI_CLOCK_HZ),
            clocks,
        );
        let timeout = F405TimeoutConfig::new(PROBE_TIMEOUT, PROBE_POLLS_PER_TIMEOUT_TICK);
        Some(Self {
            uart: F405Uart::new(serial, timeout, uart_config),
            spi: F405Spi::new(spi, timeout),
            uart_config,
        })
    }

    /// Runs one bounded no-peer receive test on each initialized peripheral.
    pub(crate) fn run(&mut self) -> F405DriverProbeResult {
        let mut uart_byte = [0; PROBE_BUFFER_LENGTH];
        let uart_timed_out = self.run_uart_probe(&mut uart_byte);
        let mut spi_byte = [PROBE_SPI_FILL_BYTE; PROBE_BUFFER_LENGTH];
        let spi_timed_out = self.run_spi_probe(&mut spi_byte);
        F405DriverProbeResult {
            uart_timed_out,
            spi_timed_out,
        }
    }

    /// Runs the owned UART timeout probe and always releases the UART.
    fn run_uart_probe(&mut self, buffer: &mut [u8; PROBE_BUFFER_LENGTH]) -> bool {
        if self.uart.acquire().is_err() {
            return false;
        }
        let timed_out = self.uart.configure(self.uart_config).is_ok()
            && matches!(
                self.uart.read(buffer, PROBE_TIMEOUT),
                Err(dali_driver_api::DriverError::Timeout)
            );
        timed_out && self.uart.release().is_ok()
    }

    /// Runs the logically selected SPI timeout probe and balances its state.
    fn run_spi_probe(&mut self, buffer: &mut [u8; PROBE_BUFFER_LENGTH]) -> bool {
        if self.spi.acquire().is_err() {
            return false;
        }
        let selected = self
            .spi
            .select(SpiDeviceId::new(PROBE_SPI_DEVICE_ID))
            .is_ok();
        if !selected {
            let _ = self.spi.release();
            return false;
        }
        let timed_out = selected
            && matches!(
                self.spi.transfer(buffer, PROBE_TIMEOUT),
                Err(dali_driver_api::DriverError::Timeout)
            );
        let deselected = selected && self.spi.deselect().is_ok();
        timed_out && deselected && self.spi.release().is_ok()
    }
}
