//! STM32F405 independent-watchdog backend.

use dali_kernel_api::{ResetCause, WatchdogBackend};
use fugit::MillisDurationU32;
use stm32f4xx_hal::{pac, watchdog::IndependentWatchdog};

const IWDG_CONTROLLER: &str = "IWDG";
const MAX_HAL_TIMEOUT_MS: u32 = 32_767;

/// Errors raised while configuring the F405 IWDG peripheral.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum F405WatchdogError {
    /// The target profile selected a controller not implemented by this backend.
    UnsupportedController,
    /// The timeout exceeds the HAL-supported IWDG range.
    TimeoutOutOfRange,
    /// Test-only failure used to verify the armed IWDG reset path on F405.
    #[cfg(feature = "watchdog-feed-failure-test")]
    FeedFailureInjected,
}

/// F405 IWDG backend with the reset cause captured during early bootstrap.
pub struct F405Watchdog {
    watchdog: IndependentWatchdog,
    reset_cause: ResetCause,
    #[cfg(feature = "watchdog-feed-failure-test")]
    feed_failure_pending: bool,
}

impl F405Watchdog {
    /// Creates an unarmed backend after reset flags have been sampled.
    pub fn new(watchdog: pac::IWDG, reset_cause: ResetCause) -> Self {
        Self {
            watchdog: IndependentWatchdog::new(watchdog),
            reset_cause,
            #[cfg(feature = "watchdog-feed-failure-test")]
            feed_failure_pending: reset_cause != ResetCause::Watchdog,
        }
    }

    /// Reads reset flags before normal bootstrap clears their latch.
    pub fn read_reset_cause(rcc: &pac::RCC) -> ResetCause {
        let status = rcc.csr.read();
        if status.wdgrstf().bit_is_set() {
            ResetCause::Watchdog
        } else if status.sftrstf().bit_is_set() {
            ResetCause::Software
        } else if status.padrstf().bit_is_set() {
            ResetCause::External
        } else if status.porrstf().bit_is_set() {
            ResetCause::PowerOn
        } else {
            ResetCause::Unknown
        }
    }

    /// Clears the latched F405 reset flags after they have been captured.
    pub fn clear_reset_cause(rcc: &pac::RCC) {
        rcc.csr.modify(|_, writer| writer.rmvf().set_bit());
    }
}

impl WatchdogBackend for F405Watchdog {
    type Error = F405WatchdogError;

    fn arm(&mut self, profile: dali_targets::WatchdogProfile) -> Result<(), Self::Error> {
        if profile.controller != IWDG_CONTROLLER {
            return Err(F405WatchdogError::UnsupportedController);
        }
        if profile.timeout_ms > MAX_HAL_TIMEOUT_MS {
            return Err(F405WatchdogError::TimeoutOutOfRange);
        }
        self.watchdog
            .start(MillisDurationU32::from_ticks(profile.timeout_ms));
        Ok(())
    }

    fn feed(&mut self) -> Result<(), Self::Error> {
        #[cfg(feature = "watchdog-feed-failure-test")]
        if self.feed_failure_pending {
            self.feed_failure_pending = false;
            return Err(F405WatchdogError::FeedFailureInjected);
        }

        self.watchdog.feed();
        Ok(())
    }

    fn reset_cause(&self) -> ResetCause {
        self.reset_cause
    }

    fn clear_reset_cause(&mut self) {
        self.reset_cause = ResetCause::Unknown;
    }
}
