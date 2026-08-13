//! RTT logging backend.

use core::fmt::Arguments;

use super::Level;

#[cfg(feature = "log-colors")]
const COLOR_RESET: &str = "\x1b[0m";
#[cfg(feature = "log-colors")]
const COLOR_RED: &str = "\x1b[31m";
#[cfg(feature = "log-colors")]
const COLOR_YELLOW: &str = "\x1b[33m";
#[cfg(feature = "log-colors")]
const COLOR_GREEN: &str = "\x1b[32m";
#[cfg(feature = "log-colors")]
const COLOR_BLUE: &str = "\x1b[34m";
#[cfg(feature = "log-colors")]
const COLOR_DIM: &str = "\x1b[2m";

impl Level {
    fn label(self) -> &'static str {
        match self {
            Self::Error => "ERROR",
            Self::Warn => "WARN",
            Self::Info => "INFO",
            Self::Debug => "DEBUG",
            Self::Trace => "TRACE",
        }
    }

    #[cfg(feature = "log-colors")]
    fn color(self) -> &'static str {
        match self {
            Self::Error => COLOR_RED,
            Self::Warn => COLOR_YELLOW,
            Self::Info => COLOR_GREEN,
            Self::Debug => COLOR_BLUE,
            Self::Trace => COLOR_DIM,
        }
    }
}

/// Initializes the RTT control block and print channel.
pub(super) fn initialize() {
    rtt_target::rtt_init_print!();
}

/// Writes one message through the non-blocking RTT channel.
pub(super) fn write(level: Level, subsystem: &'static str, arguments: Arguments<'_>) {
    #[cfg(feature = "log-colors")]
    rtt_target::rprintln!(
        "{}[{}][{}] {}{}",
        level.color(),
        level.label(),
        subsystem,
        arguments,
        COLOR_RESET
    );

    #[cfg(not(feature = "log-colors"))]
    rtt_target::rprintln!("[{}][{}] {}", level.label(), subsystem, arguments);
}
