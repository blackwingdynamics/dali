use dali_driver_api::{
    BoundedTimeout, CountDown, DriverError, DriverResult, Duration, TimerDriver,
};

pub struct MockTimer {
    pub max: Duration,
    pub active: bool,
    pub expired: bool,
    pub cancel_count: u8,
}

impl MockTimer {
    pub const fn new(max: Duration) -> Self {
        Self {
            max,
            active: false,
            expired: false,
            cancel_count: 0,
        }
    }

    pub fn validate(&self, timeout: Duration) -> DriverResult<()> {
        BoundedTimeout::validate_timeout(self, timeout)
    }
}

impl BoundedTimeout for MockTimer {
    fn max_timeout(&self) -> Duration {
        self.max
    }
}

impl CountDown for MockTimer {
    fn start(&mut self, timeout: Duration) -> DriverResult<()> {
        self.validate(timeout)?;
        self.active = true;
        self.expired = false;
        Ok(())
    }

    fn wait(&mut self) -> DriverResult<bool> {
        if !self.active {
            return Err(DriverError::InvalidState);
        }
        Ok(self.expired)
    }

    fn cancel(&mut self) -> DriverResult<()> {
        if !self.active {
            return Err(DriverError::InvalidState);
        }
        self.active = false;
        self.cancel_count = self.cancel_count.saturating_add(1);
        Ok(())
    }
}

impl TimerDriver for MockTimer {
    fn start(&mut self, timeout: Duration) -> DriverResult<()> {
        CountDown::start(self, timeout)
    }

    fn stop(&mut self) -> DriverResult<()> {
        if !self.active {
            return Err(DriverError::InvalidState);
        }
        self.active = false;
        Ok(())
    }

    fn is_running(&self) -> DriverResult<bool> {
        Ok(self.active)
    }

    fn is_expired(&mut self) -> DriverResult<bool> {
        if !self.active {
            return Err(DriverError::InvalidState);
        }
        Ok(self.expired)
    }
}
