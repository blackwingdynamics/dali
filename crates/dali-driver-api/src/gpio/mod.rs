//! Allocation-free GPIO contracts.

mod interrupt;
mod pins;

pub use interrupt::{InterruptCallback, InterruptPin, InterruptTrigger};
pub use pins::{GpioMode, InputPin, OutputPin, PinMode};
