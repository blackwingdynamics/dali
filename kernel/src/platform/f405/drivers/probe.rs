//! F405 bounded-I/O acceptance probe.

use super::{F405Spi, F405TimeoutConfig, F405Uart};
use dali_driver_api::{Duration, SerialRead, SpiTransfer};
use stm32f4xx_hal::{
    gpio::{self, Alternate},
    pac,
    prelude::*,
    rcc::Clocks,
    serial::{Config as SerialConfig, SerialExt},
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
        let serial = match usart.serial(
            uart_pins,
            SerialConfig::default().baudrate(PROBE_UART_BAUD_HZ.bps()),
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
            uart: F405Uart::new(serial, timeout),
            spi: F405Spi::new(spi, timeout),
        })
    }

    /// Runs one bounded no-peer receive test on each initialized peripheral.
    pub(crate) fn run(&mut self) -> F405DriverProbeResult {
        let mut uart_byte = [0; PROBE_BUFFER_LENGTH];
        let uart_timed_out = matches!(
            self.uart.read(&mut uart_byte, PROBE_TIMEOUT),
            Err(dali_driver_api::DriverError::Timeout)
        );
        let mut spi_byte = [PROBE_SPI_FILL_BYTE; PROBE_BUFFER_LENGTH];
        let spi_timed_out = matches!(
            self.spi.transfer(&mut spi_byte, PROBE_TIMEOUT),
            Err(dali_driver_api::DriverError::Timeout)
        );
        F405DriverProbeResult {
            uart_timed_out,
            spi_timed_out,
        }
    }
}
