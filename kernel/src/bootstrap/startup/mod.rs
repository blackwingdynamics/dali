//! Hardware-independent startup service initialization.

mod logging;
#[cfg(feature = "sdio")]
mod watchdog;

pub(super) use logging::{emit_boot_banner, initialize as initialize_logging};
#[cfg(feature = "sdio")]
pub(super) use watchdog::install as install_watchdog;
