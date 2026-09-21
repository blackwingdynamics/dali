//! Board-local diagnostic output used during backend bring-up.

use core::fmt::Arguments;

/// Subsystem label for board bootstrap diagnostics.
pub const BOOT_SUBSYSTEM: &str = "BOOT";

/// Writes an informational board diagnostic.
#[cfg(feature = "driver-hardware-test")]
pub fn info(subsystem: &str, arguments: Arguments<'_>) {
    rtt_target::rprintln!("[INFO][{}] {}", subsystem, arguments);
}

/// Writes an error board diagnostic.
pub fn error(subsystem: &str, arguments: Arguments<'_>) {
    rtt_target::rprintln!("[ERROR][{}] {}", subsystem, arguments);
}
