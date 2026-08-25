//! F405 SysTick adapter kept behind the platform boundary.

use dali_driver_api::{BoundedTimeout, DriverError, DriverResult, Duration, TimerDriver};
use stm32f4xx_hal::{
    time::Hertz,
    timer::{SysCounterHz, SysEvent},
};

/// SysTick-backed timer adapter with manifest-provided timing units.
pub(crate) struct F405TimerDriver {
    counter: SysCounterHz,
    ticks_per_second: u32,
    maximum_timeout: Duration,
    running: bool,
}

impl F405TimerDriver {
    /// Creates an adapter over the board's exclusively owned SysTick counter.
    pub(crate) fn new(
        counter: SysCounterHz,
        ticks_per_second: u32,
        maximum_timeout: Duration,
    ) -> Self {
        Self {
            counter,
            ticks_per_second,
            maximum_timeout,
            running: false,
        }
    }

    /// Starts the periodic scheduler frequency selected by the platform.
    pub(crate) fn start_frequency(&mut self, frequency_hz: u32) -> DriverResult<()> {
        if frequency_hz == 0 {
            return Err(DriverError::Timeout);
        }
        self.counter
            .start(Hertz::from_raw(frequency_hz))
            .map_err(|_| DriverError::HardwareFault)?;
        self.running = true;
        Ok(())
    }

    /// Enables the SysTick update event after the timer is configured.
    pub(crate) fn listen_update(&mut self) {
        self.counter.listen(SysEvent::Update);
    }

    /// Polls one bounded interval for a real SysTick wrap during acceptance.
    #[cfg(feature = "driver-hardware-test")]
    pub(crate) fn wait_for_tick(&mut self, poll_limit: u32) -> bool {
        if !self.running || poll_limit == 0 {
            return false;
        }
        let mut polls = 0;
        while polls < poll_limit {
            if matches!(self.is_expired(), Ok(true)) {
                return true;
            }
            polls += 1;
        }
        false
    }

    /// Waits for one configured timer period within a finite poll budget.
    #[cfg(feature = "driver-hardware-test")]
    pub(crate) fn wait_for_timeout(&mut self, poll_limit: u32) -> bool {
        if poll_limit == 0 {
            return false;
        }
        let mut polls = 0;
        while polls < poll_limit {
            match self.is_expired() {
                Ok(true) => return true,
                Ok(false) => polls += 1,
                Err(_) => return false,
            }
        }
        false
    }

    fn timeout_frequency(&self, timeout: Duration) -> DriverResult<u32> {
        self.validate_timeout(timeout)?;
        let frequency = self
            .ticks_per_second
            .checked_div(timeout.ticks())
            .ok_or(DriverError::Timeout)?;
        if frequency == 0 {
            Err(DriverError::Timeout)
        } else {
            Ok(frequency)
        }
    }
}

impl BoundedTimeout for F405TimerDriver {
    fn max_timeout(&self) -> Duration {
        self.maximum_timeout
    }
}

impl TimerDriver for F405TimerDriver {
    fn start(&mut self, timeout: Duration) -> DriverResult<()> {
        let frequency = self.timeout_frequency(timeout)?;
        self.start_frequency(frequency)
    }

    fn stop(&mut self) -> DriverResult<()> {
        if !self.running {
            return Err(DriverError::InvalidState);
        }
        self.counter
            .cancel()
            .map_err(|_| DriverError::HardwareFault)?;
        self.running = false;
        Ok(())
    }

    fn is_running(&self) -> DriverResult<bool> {
        Ok(self.running)
    }

    fn is_expired(&mut self) -> DriverResult<bool> {
        if !self.running {
            return Err(DriverError::InvalidState);
        }
        Ok(self.counter.wait().is_ok())
    }
}
