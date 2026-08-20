//! Kernel startup orchestration.

mod lifecycle;
mod loading;
mod startup;
mod storage;

use crate::{logging, platform};

/// Runs the kernel bootstrap sequence and enters the heartbeat loop.
pub fn run() -> ! {
    let mut board = platform::initialize();
    #[cfg(feature = "abi-mpu")]
    if let Some(layout) = platform::ISOLATION_LAYOUT {
        platform::mpu::configure_hardware(layout);
    }

    startup::initialize_logging();
    let reset_cause = board.reset_cause();
    logging::info(
        logging::BOOT_SUBSYSTEM,
        format_args!("[BOOT] Reset cause: {:?}", reset_cause),
    );
    let boot_mode = lifecycle::status::BootMode::from_reset_cause(reset_cause);
    if boot_mode == lifecycle::status::BootMode::SafeMode {
        logging::error(
            logging::SECURITY_SUBSYSTEM,
            format_args!(
                "[RECOVERY] Watchdog reset detected; entering Safe Mode and skipping application launch"
            ),
        );
    }
    #[cfg(feature = "usb-cdc")]
    if let Some(resources) = board.take_usb_resources() {
        logging::initialize_usb(resources);
    } else {
        logging::error(
            logging::BOOT_SUBSYSTEM,
            format_args!("[USB] USB resources unavailable"),
        );
    }

    startup::emit_boot_banner();
    logging::info(
        logging::BOOT_SUBSYSTEM,
        format_args!("[BOOT] System clock: {} MHz", platform::SYSTEM_CLOCK_MHZ),
    );
    logging::info(
        logging::BOOT_SUBSYSTEM,
        format_args!("Hardware bootstrap complete"),
    );

    let watchdog = startup::initialize_watchdog(&mut board);

    #[cfg(feature = "abi-context-switch")]
    if let Err(error) = crate::security::scheduling::initialize() {
        logging::error(
            logging::SECURITY_SUBSYSTEM,
            format_args!("[SECURITY] Scheduler initialization failed: {:?}", error),
        );
    }

    // Keep a visible indication active while storage initialization is in progress.
    board.set_status_led(true);
    logging::info(
        logging::BOOT_SUBSYSTEM,
        format_args!("[STORAGE] Starting storage initialization"),
    );
    let storage_status = storage::initialize(&mut board, boot_mode);

    logging::info(
        logging::BOOT_SUBSYSTEM,
        format_args!("Entering kernel heartbeat"),
    );

    lifecycle::heartbeat::run(board, storage_status, watchdog);
}
