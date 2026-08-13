//! Kernel-wide logging facade.

use core::fmt::Arguments;

#[cfg(feature = "usb-cdc")]
mod buffer;
mod rtt;
#[cfg(feature = "usb-cdc")]
pub(crate) mod usb_cdc;

#[cfg(feature = "usb-cdc")]
use buffer::{LOG_LINE_CAPACITY, LogLine, LogQueue};

#[cfg(feature = "usb-cdc")]
static mut USB_LOG_QUEUE: LogQueue = LogQueue::new();

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
    {
        usb_cdc::poll();
        drain_usb_queue();
    }
}

/// Writes a structured, allocation-free message to the selected backend.
pub fn log(level: Level, subsystem: &'static str, arguments: Arguments<'_>) {
    rtt::write(level, subsystem, arguments);
    #[cfg(feature = "usb-cdc")]
    {
        if let Ok(line) = LogLine::format(level, subsystem, arguments) {
            // SAFETY: The kernel has one execution context and USB logging is
            // not interrupt-driven. The queue is accessed only by this context.
            unsafe { (*core::ptr::addr_of_mut!(USB_LOG_QUEUE)).push(line) };
        }
        usb_cdc::poll();
        drain_usb_queue();
    }
}

#[cfg(feature = "usb-cdc")]
fn drain_usb_queue() {
    if !usb_cdc::host_ready() {
        return;
    }

    let mut chunk = [0; LOG_LINE_CAPACITY];
    loop {
        // SAFETY: The queue is owned by the single kernel execution context.
        let count = unsafe { (*core::ptr::addr_of!(USB_LOG_QUEUE)).copy_front(&mut chunk) };
        if count == 0 {
            break;
        }
        let written = usb_cdc::write_bytes(&chunk[..count]);
        if written == 0 {
            return;
        }
        // SAFETY: The queue remains exclusively owned by this execution context.
        unsafe { (*core::ptr::addr_of_mut!(USB_LOG_QUEUE)).advance(written) };
    }

    // The queue intentionally keeps the newest messages when its fixed capacity
    // is exhausted. Emit a visible diagnostic after the queue becomes writable.
    let dropped = unsafe { (*core::ptr::addr_of_mut!(USB_LOG_QUEUE)).take_dropped() };
    if dropped > 0 {
        let _ = usb_cdc::write_bytes(
            b"[WARN][LOG] USB log queue overflow; oldest messages were dropped\r\n",
        );
    }
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
