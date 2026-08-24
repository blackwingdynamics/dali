//! Kernel logging startup and boot banner.

use crate::logging;

/// Initializes the kernel logging backend during bootstrap.
pub fn initialize() {
    logging::initialize();
}

/// Performs the `emit_boot_banner` operation for this subsystem.
pub fn emit_boot_banner() {
    logging::info(
        logging::BOOT_SUBSYSTEM,
        format_args!("===================================="),
    );
    logging::info(
        logging::BOOT_SUBSYSTEM,
        format_args!("   Dali OS Kernel Booting...       "),
    );
    logging::info(
        logging::BOOT_SUBSYSTEM,
        format_args!("===================================="),
    );
}
