//! F405 GPIO adapter kept behind the platform boundary.

use dali_driver_api::{
    DriverError, DriverResult, GpioMode, InputPin, InterruptPin, InterruptTrigger, OutputPin,
    PinMode,
};
use stm32f4xx_hal::gpio::{DynamicPin, PinState};

/// GPIO adapter that owns one dynamically configurable F405 pin.
pub(crate) struct F405GpioPin<const PORT: char, const PIN: u8> {
    pin: DynamicPin<PORT, PIN>,
    mode: GpioMode,
    output_high: bool,
}

impl<const PORT: char, const PIN: u8> F405GpioPin<PORT, PIN> {
    /// Wraps and configures a HAL dynamic pin as a push-pull output.
    pub(crate) fn new_output(mut pin: DynamicPin<PORT, PIN>) -> Self {
        pin.make_push_pull_output();
        Self {
            pin,
            mode: GpioMode::Output,
            output_high: false,
        }
    }

    fn map_pin_error(_: impl Copy) -> DriverError {
        DriverError::InvalidState
    }

    fn require_output(&self) -> DriverResult<()> {
        if self.mode == GpioMode::Output {
            Ok(())
        } else {
            Err(DriverError::InvalidState)
        }
    }
}

impl<const PORT: char, const PIN: u8> InputPin for F405GpioPin<PORT, PIN> {
    fn is_high(&self) -> DriverResult<bool> {
        self.pin.is_high().map_err(Self::map_pin_error)
    }
}

impl<const PORT: char, const PIN: u8> OutputPin for F405GpioPin<PORT, PIN> {
    fn set_high(&mut self) -> DriverResult<()> {
        self.require_output()?;
        self.pin.set_high().map_err(Self::map_pin_error)?;
        self.output_high = true;
        Ok(())
    }

    fn set_low(&mut self) -> DriverResult<()> {
        self.require_output()?;
        self.pin.set_low().map_err(Self::map_pin_error)?;
        self.output_high = false;
        Ok(())
    }

    fn toggle(&mut self) -> DriverResult<()> {
        if self.output_high {
            self.set_low()
        } else {
            self.set_high()
        }
    }
}

impl<const PORT: char, const PIN: u8> PinMode for F405GpioPin<PORT, PIN> {
    fn set_mode(&mut self, mode: GpioMode) -> DriverResult<()> {
        match mode {
            GpioMode::Input => self.pin.make_floating_input(),
            GpioMode::Output => {
                let state = if self.output_high {
                    PinState::High
                } else {
                    PinState::Low
                };
                self.pin.make_push_pull_output_in_state(state);
            }
            GpioMode::Alternate => return Err(DriverError::Unsupported),
        }
        self.mode = mode;
        Ok(())
    }
}

impl<const PORT: char, const PIN: u8> InterruptPin for F405GpioPin<PORT, PIN> {
    fn enable_interrupt(&mut self, _: InterruptTrigger) -> DriverResult<()> {
        Err(DriverError::Unsupported)
    }

    fn disable_interrupt(&mut self) -> DriverResult<()> {
        Err(DriverError::Unsupported)
    }

    fn take_pending(&mut self) -> DriverResult<bool> {
        Err(DriverError::Unsupported)
    }
}
