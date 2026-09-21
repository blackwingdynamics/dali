//! F405 EXTI adapter kept behind the platform boundary.

use dali_driver_api::{
    DriverError, DriverResult, InputPin, InterruptCallback, InterruptPin, InterruptTrigger,
};
use stm32f4xx_hal::{
    gpio::{Edge, ExtiPin as HalExtiPin, Input, Pin},
    pac::{self, EXTI},
    syscfg::SysCfg,
};

/// Concrete F405 input pin bound to one EXTI line and its callback hook.
pub struct F405ExtiPin<const PORT: char, const PIN: u8> {
    /// Stores the pin associated with this bounded state.
    pin: Pin<PORT, PIN, Input>,
    /// Stores the exti associated with this bounded state.
    exti: EXTI,
    /// Stores the callback associated with this bounded state.
    callback: Option<InterruptCallback>,
    /// Stores the enabled associated with this bounded state.
    enabled: bool,
}

impl<const PORT: char, const PIN: u8> F405ExtiPin<PORT, PIN> {
    /// Performs the `helper` operation for this subsystem.
    ///
    /// Arguments select the bounded state, buffer, or hardware operation described by the signature.
    const PENDING_MASK: u32 = 1_u32 << PIN;

    /// Binds a floating/input pin to the platform-owned EXTI and SYSCFG.
    pub fn new(mut pin: Pin<PORT, PIN, Input>, exti: pac::EXTI, mut syscfg: SysCfg) -> Self {
        pin.make_interrupt_source(&mut syscfg);
        Self {
            pin,
            exti,
            callback: None,
            enabled: false,
        }
    }

    /// Performs the `edge` operation for this subsystem.
    ///
    /// Arguments select the bounded state, buffer, or hardware operation described by the signature.
    ///
    /// # Errors
    /// Returns a typed error when validation, state, or hardware access fails.
    fn edge(trigger: InterruptTrigger) -> DriverResult<Edge> {
        match trigger {
            InterruptTrigger::RisingEdge => Ok(Edge::Rising),
            InterruptTrigger::FallingEdge => Ok(Edge::Falling),
            InterruptTrigger::BothEdges => Ok(Edge::RisingFalling),
            InterruptTrigger::LevelHigh | InterruptTrigger::LevelLow => {
                Err(DriverError::Unsupported)
            }
        }
    }

    /// Polls and acknowledges the EXTI pending register for this input line.
    pub(crate) fn poll_pending_register(&mut self) -> DriverResult<bool> {
        if !self.enabled {
            return Err(DriverError::InvalidState);
        }
        let pending = (self.exti.pr.read().bits() & Self::PENDING_MASK) != 0;
        if pending {
            self.exti.pr.write(|writer| unsafe {
                // SAFETY: PENDING_MASK contains only this validated EXTI line;
                // STM32 EXTI PR uses write-one-to-clear semantics.
                writer.bits(Self::PENDING_MASK)
            });
        }
        Ok(pending)
    }
}

#[cfg(feature = "driver-hardware-test")]
impl<const PORT: char, const PIN: u8> F405ExtiPin<PORT, PIN> {
    /// Polls and acknowledges EXTI13 when the IRQ path did not service it.
    pub(crate) fn poll_user_key_trigger(&mut self) -> DriverResult<bool> {
        if !self.enabled {
            return Err(DriverError::InvalidState);
        }
        let pending = (self.exti.pr.read().bits() & Self::PENDING_MASK) != 0;
        if pending {
            self.exti.pr.write(|writer| unsafe {
                // SAFETY: PENDING_MASK contains only EXTI13; EXTI PR is
                // write-one-to-clear on STM32F405.
                writer.bits(Self::PENDING_MASK)
            });
        }
        Ok(pending)
    }
}

impl<const PORT: char, const PIN: u8> InputPin for F405ExtiPin<PORT, PIN> {
    fn is_high(&self) -> DriverResult<bool> {
        Ok(self.pin.is_high())
    }
}

impl<const PORT: char, const PIN: u8> InterruptPin for F405ExtiPin<PORT, PIN> {
    fn enable_interrupt(
        &mut self,
        trigger: InterruptTrigger,
        callback: Option<InterruptCallback>,
    ) -> DriverResult<()> {
        let edge = Self::edge(trigger)?;
        self.pin.trigger_on_edge(&mut self.exti, edge);
        self.pin.enable_interrupt(&mut self.exti);
        self.callback = callback;
        self.enabled = true;
        Ok(())
    }

    fn disable_interrupt(&mut self) -> DriverResult<()> {
        if !self.enabled {
            return Err(DriverError::InvalidState);
        }
        self.pin.disable_interrupt(&mut self.exti);
        self.callback = None;
        self.enabled = false;
        Ok(())
    }

    fn take_pending(&mut self) -> DriverResult<bool> {
        let pending = self.poll_pending_register()?;
        if pending && let Some(callback) = self.callback {
            callback();
        }
        Ok(pending)
    }
}
