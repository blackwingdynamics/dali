//! Bounded timer and countdown contracts.

mod countdown;
mod driver;

pub use countdown::{BoundedTimeout, CountDown, Duration};
pub use driver::TimerDriver;
