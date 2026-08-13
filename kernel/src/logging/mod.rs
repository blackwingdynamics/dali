//! Kernel-wide logging facade.

use core::fmt::Arguments;

mod rtt;

/// Stable subsystem label for bootstrap messages.
pub const BOOT_SUBSYSTEM: &str = "BOOT";

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

/// Initializes the selected logging backend.
pub fn initialize() {
    rtt::initialize();
}

/// Writes a structured, allocation-free message to the selected backend.
pub fn log(level: Level, subsystem: &'static str, arguments: Arguments<'_>) {
    rtt::write(level, subsystem, arguments);
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
