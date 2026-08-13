//! Kernel-wide logging facade.

use core::fmt::Arguments;

mod rtt;
#[cfg(feature = "usb-cdc")]
pub(crate) mod usb_cdc;

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

impl Level {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Error => "ERROR",
            Self::Warn => "WARN",
            Self::Info => "INFO",
            Self::Debug => "DEBUG",
            Self::Trace => "TRACE",
        }
    }
}

/// Initializes the selected logging backend.
pub fn initialize() {
    rtt::initialize();
}

/// Initializes the optional USB CDC logging backend.
#[cfg(feature = "usb-cdc")]
pub fn initialize_usb(resources: crate::board::UsbResources) {
    usb_cdc::initialize(resources);
}

/// Services the optional USB CDC device state machine.
pub fn poll() {
    #[cfg(feature = "usb-cdc")]
    usb_cdc::poll();
}

/// Writes a structured, allocation-free message to the selected backend.
pub fn log(level: Level, subsystem: &'static str, arguments: Arguments<'_>) {
    rtt::write(level, subsystem, arguments);
    #[cfg(feature = "usb-cdc")]
    usb_cdc::write(level, subsystem, arguments);
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
