//! Kernel-wide logging facade.

use core::fmt::Arguments;

#[cfg(feature = "usb-cdc")]
/// Groups the private helpers for this subsystem.
mod buffer {
    /// Defines the `LOG_LINE_CAPACITY` bound used by this subsystem.
    pub(crate) const LOG_LINE_CAPACITY: usize = 128;
    /// Defines the `LOG_QUEUE_CAPACITY` bound used by this subsystem.
    pub(crate) const LOG_QUEUE_CAPACITY: usize = 64;
    /// Names the bounded `LogLine` type used by this subsystem.
    pub(crate) type LogLine = dali_usb::LogLine<LOG_LINE_CAPACITY>;
    /// Names the bounded `LogQueue` type used by this subsystem.
    pub(crate) type LogQueue = dali_usb::LogQueue<LOG_LINE_CAPACITY, LOG_QUEUE_CAPACITY>;
}
mod rtt;
#[cfg(feature = "usb-cdc")]
use buffer::{LogLine, LogQueue};

#[cfg(feature = "usb-cdc")]
/// Stores the shared `mut` state used by this subsystem.
static mut USB_LOG_QUEUE: LogQueue = LogQueue::new();

#[cfg(feature = "usb-cdc")]
/// Defines the `USB_OVERFLOW_WARNING` bound used by this subsystem.
const USB_OVERFLOW_WARNING: &str =
    "[WARN][LOG] USB log queue overflow; oldest messages were dropped\r\n";
#[cfg(feature = "usb-cdc")]
/// Defines the `USB_FLUSH_FAILURE_WARNING` bound used by this subsystem.
const USB_FLUSH_FAILURE_WARNING: &str =
    "[WARN][USB] CDC transport flush failed; queued delivery may be delayed\r\n";

/// Stable subsystem label for bootstrap messages.
pub const BOOT_SUBSYSTEM: &str = "BOOT";
/// Subsystem label for messages submitted by a native application.
pub const APPLICATION_SUBSYSTEM: &str = "APP";
/// Subsystem label for kernel security-boundary events.
pub const SECURITY_SUBSYSTEM: &str = "SECURITY";

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
    /// Performs the `label` operation for this subsystem.
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

/// Services the optional USB CDC device state machine from its interrupt.
#[cfg(feature = "usb-cdc")]
pub(crate) fn service_usb_irq() {
    crate::platform::service_usb_irq(drain_usb_queue, crate::installation::receive_bytes);
}

/// Entry point called by the selected board's USB interrupt wrapper.
#[cfg(feature = "usb-cdc")]
#[unsafe(export_name = "dali_kernel_service_usb_irq")]
pub extern "C" fn dali_kernel_service_usb_irq() {
    service_usb_irq();
}

/// Writes a structured, allocation-free message to the selected backend.
pub fn log(level: Level, subsystem: &'static str, arguments: Arguments<'_>) {
    rtt::write(level, subsystem, arguments);
    #[cfg(feature = "usb-cdc")]
    {
        if let Ok(line) = LogLine::format(format_args!(
            "[{}][{}] {}\r\n",
            level.label(),
            subsystem,
            arguments
        )) {
            critical_section::with(|_| {
                // SAFETY: Main-context logging masks the USB interrupt while updating the
                // queue, so the handler cannot observe a partial record.
                unsafe { (*core::ptr::addr_of_mut!(USB_LOG_QUEUE)).push(line) };
                crate::platform::pend_usb_irq();
            });
        }
    }
}

#[cfg(feature = "usb-cdc")]
/// Performs the `drain_usb_queue` operation for this subsystem.
fn drain_usb_queue(link: dali_usb::LinkState, sink: &mut dyn dali_usb::ByteSink) {
    // SAFETY: This runs in the USB handler while main-context queue updates
    // are masked by the interrupt-free critical section.
    let report =
        unsafe { dali_usb::drain(&mut *core::ptr::addr_of_mut!(USB_LOG_QUEUE), link, sink) };

    if report.flush == dali_usb::FlushStatus::Failed {
        rtt::write(
            Level::Warn,
            BOOT_SUBSYSTEM,
            format_args!("{}", USB_FLUSH_FAILURE_WARNING),
        );
    }

    let dropped = report.dropped;

    // The queue intentionally keeps the newest messages when its fixed capacity
    // is exhausted. Emit a visible diagnostic after the queue becomes writable.
    if dropped > 0
        && let Some(warning) = LogLine::from_bytes(USB_OVERFLOW_WARNING.as_bytes())
    {
        // The queue is empty after the first drain, so the warning remains
        // queued if the endpoint accepts only part of it or disconnects.
        // SAFETY: This function runs only in the USB handler; main-context
        // queue updates are masked while the handler cannot be preempted.
        unsafe { (*core::ptr::addr_of_mut!(USB_LOG_QUEUE)).push(warning) };
        // SAFETY: The same USB ownership invariant applies while the
        // warning is drained immediately after it is queued.
        let queue = unsafe { &mut *core::ptr::addr_of_mut!(USB_LOG_QUEUE) };
        let report = dali_usb::drain(queue, link, sink);
        if report.flush == dali_usb::FlushStatus::Failed {
            rtt::write(
                Level::Warn,
                BOOT_SUBSYSTEM,
                format_args!("{}", USB_FLUSH_FAILURE_WARNING),
            );
        }
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
