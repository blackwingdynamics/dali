//! Hardware-independent startup service initialization.

mod logging;
mod watchdog;

pub(super) use logging::{emit_boot_banner, initialize as initialize_logging};
pub(super) use watchdog::initialize as initialize_watchdog;
