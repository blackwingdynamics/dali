//! F405 hardware acceptance probe orchestration and diagnostics.

use super::super::drivers::F405DriverProbeResult;
use super::resources::Board;

impl Board {
    /// Runs the opt-in UART, SPI, and I2C bounded-timeout acceptance probe.
    #[cfg(feature = "driver-hardware-test")]
    pub(crate) fn run_driver_timeout_probe(&mut self) {
        let Some(probe) = self.driver_probe.as_mut() else {
            crate::logging::error(
                crate::logging::BOOT_SUBSYSTEM,
                format_args!("[DRIVER] UART/SPI probe initialization failed"),
            );
            return;
        };
        #[cfg(feature = "display-oled")]
        let i2c = self.display.as_mut().map(|display| display.bus_mut());
        #[cfg(not(feature = "display-oled"))]
        let i2c = self.i2c.as_mut();
        let F405DriverProbeResult {
            uart_timed_out,
            spi_timed_out,
            spi_recovered,
            i2c_completed,
            i2c_recovered,
        } = probe.run(i2c);
        log_uart_probe_result(uart_timed_out);
        log_spi_probe_result(spi_timed_out, spi_recovered);
        log_i2c_probe_result(i2c_completed, i2c_recovered);
    }
}

#[cfg(feature = "driver-hardware-test")]
fn log_uart_probe_result(timed_out: bool) {
    if timed_out {
        crate::logging::info(
            crate::logging::BOOT_SUBSYSTEM,
            format_args!("[DRIVER][UART] Bounded timeout enforced"),
        );
    } else {
        crate::logging::error(
            crate::logging::BOOT_SUBSYSTEM,
            format_args!("[DRIVER][UART] Timeout probe did not time out"),
        );
    }
}

#[cfg(feature = "driver-hardware-test")]
fn log_spi_probe_result(timed_out: bool, recovered: bool) {
    if timed_out {
        crate::logging::info(
            crate::logging::BOOT_SUBSYSTEM,
            format_args!("[DRIVER][SPI] Stalled transfer timeout enforced"),
        );
    } else {
        crate::logging::error(
            crate::logging::BOOT_SUBSYSTEM,
            format_args!("[DRIVER][SPI] Stalled transfer did not time out"),
        );
    }
    if recovered {
        crate::logging::info(
            crate::logging::BOOT_SUBSYSTEM,
            format_args!("[DRIVER][SPI] Recovery transfer completed"),
        );
    } else {
        crate::logging::error(
            crate::logging::BOOT_SUBSYSTEM,
            format_args!("[DRIVER][SPI] Recovery transfer failed"),
        );
    }
}

#[cfg(feature = "driver-hardware-test")]
fn log_i2c_probe_result(completed: bool, recovered: bool) {
    if completed {
        crate::logging::info(
            crate::logging::BOOT_SUBSYSTEM,
            format_args!("[DRIVER][I2C] Bounded transfer completed"),
        );
    } else {
        crate::logging::error(
            crate::logging::BOOT_SUBSYSTEM,
            format_args!("[DRIVER][I2C] Bounded transfer timeout enforced"),
        );
    }
    if recovered {
        crate::logging::info(
            crate::logging::BOOT_SUBSYSTEM,
            format_args!("[DRIVER][I2C] Bus recovered"),
        );
    }
}
