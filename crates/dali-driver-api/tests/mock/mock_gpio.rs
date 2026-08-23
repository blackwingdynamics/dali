use dali_driver_api::{
    DriverError, DriverResult, GpioMode, InputPin, InterruptCallback, InterruptPin,
    InterruptTrigger, OutputPin, PinMode,
};

pub struct MockGpio {
    pub high: bool,
    pub mode: GpioMode,
    pub pending: bool,
    pub interrupt: Option<InterruptTrigger>,
    pub callback: Option<InterruptCallback>,
    pub interrupt_supported: bool,
    pub busy: bool,
}

impl MockGpio {
    pub const fn new() -> Self {
        Self {
            high: false,
            mode: GpioMode::Input,
            pending: false,
            interrupt: None,
            callback: None,
            interrupt_supported: true,
            busy: false,
        }
    }

    fn check_available(&self) -> DriverResult<()> {
        if self.busy {
            Err(DriverError::ResourceBusy)
        } else {
            Ok(())
        }
    }
}

impl InputPin for MockGpio {
    fn is_high(&self) -> DriverResult<bool> {
        self.check_available()?;
        Ok(self.high)
    }
}

impl OutputPin for MockGpio {
    fn set_high(&mut self) -> DriverResult<()> {
        self.check_available()?;
        self.high = true;
        Ok(())
    }

    fn set_low(&mut self) -> DriverResult<()> {
        self.check_available()?;
        self.high = false;
        Ok(())
    }

    fn toggle(&mut self) -> DriverResult<()> {
        self.check_available()?;
        self.high = !self.high;
        Ok(())
    }
}

impl PinMode for MockGpio {
    fn set_mode(&mut self, mode: GpioMode) -> DriverResult<()> {
        self.check_available()?;
        self.mode = mode;
        Ok(())
    }
}

impl InterruptPin for MockGpio {
    fn enable_interrupt(
        &mut self,
        trigger: InterruptTrigger,
        callback: Option<InterruptCallback>,
    ) -> DriverResult<()> {
        self.check_available()?;
        if !self.interrupt_supported {
            return Err(DriverError::Unsupported);
        }
        self.interrupt = Some(trigger);
        self.callback = callback;
        Ok(())
    }

    fn disable_interrupt(&mut self) -> DriverResult<()> {
        self.check_available()?;
        self.interrupt = None;
        self.callback = None;
        Ok(())
    }

    fn take_pending(&mut self) -> DriverResult<bool> {
        self.check_available()?;
        let pending = self.pending;
        self.pending = false;
        if pending && let Some(callback) = self.callback {
            callback();
        }
        Ok(pending)
    }
}
