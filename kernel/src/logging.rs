//! Kernel logging boundary.

use core::fmt::Arguments;

/// Stable subsystem label for bootstrap messages.
pub const BOOT_SUBSYSTEM: &str = "BOOT";

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

/// Message severity used by the kernel logging facade.
#[derive(Clone, Copy)]
pub enum Level {
    /// A failure that prevents the current operation from succeeding.
    Error,
    /// An abnormal condition that does not stop execution immediately.
    Warn,
    /// A normal operational event.
    Info,
    /// Diagnostic information useful during development.
    Debug,
    /// Highly detailed diagnostic information.
    Trace,
}

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

/// Initializes the kernel's RTT logging channel.
pub fn initialize() {
    rtt_target::rtt_init_print!();
}

/// Writes a structured, allocation-free message to the kernel RTT channel.
pub fn log(level: Level, subsystem: &'static str, arguments: Arguments<'_>) {
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

/// Writes an informational message.
pub fn info(subsystem: &'static str, arguments: Arguments<'_>) {
    log(Level::Info, subsystem, arguments);
}

/// Writes an error message.
pub fn error(subsystem: &'static str, arguments: Arguments<'_>) {
    log(Level::Error, subsystem, arguments);
}

/// Writes a warning message.
pub fn warn(subsystem: &'static str, arguments: Arguments<'_>) {
    log(Level::Warn, subsystem, arguments);
}

/// Writes a development diagnostic message.
pub fn debug(subsystem: &'static str, arguments: Arguments<'_>) {
    log(Level::Debug, subsystem, arguments);
}

/// Writes a highly detailed diagnostic message.
pub fn trace(subsystem: &'static str, arguments: Arguments<'_>) {
    log(Level::Trace, subsystem, arguments);
}
