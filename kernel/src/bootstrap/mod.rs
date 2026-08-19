//! Kernel startup orchestration.

mod heartbeat;
mod status;
mod storage;

use crate::runtime::watchdog::WatchdogRuntime;
use crate::{logging, platform};

/// Runs the kernel bootstrap sequence and enters the heartbeat loop.
pub fn run() -> ! {
    let mut board = platform::initialize();
    #[cfg(feature = "abi-mpu")]
    if let Some(layout) = platform::ISOLATION_LAYOUT {
        platform::mpu::configure_hardware(layout);
    }

    initialize_logging();
    logging::info(
        logging::BOOT_SUBSYSTEM,
        format_args!("[BOOT] Reset cause: {:?}", board.reset_cause()),
    );
    #[cfg(feature = "usb-cdc")]
    if let Some(resources) = board.take_usb_resources() {
        logging::initialize_usb(resources);
    } else {
        logging::error(
            logging::BOOT_SUBSYSTEM,
            format_args!("[USB] USB resources unavailable"),
        );
    }

    emit_boot_banner();
    logging::info(
        logging::BOOT_SUBSYSTEM,
        format_args!("[BOOT] System clock: {} MHz", platform::SYSTEM_CLOCK_MHZ),
    );
    logging::info(
        logging::BOOT_SUBSYSTEM,
        format_args!("Hardware bootstrap complete"),
    );

    let watchdog = initialize_watchdog(&mut board);

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
    let storage_status = storage::initialize(&mut board);

    logging::info(
        logging::BOOT_SUBSYSTEM,
        format_args!("Entering kernel heartbeat"),
    );

    heartbeat::run(board, storage_status, watchdog);
}

fn initialize_watchdog(board: &mut platform::Platform) -> Option<platform::WatchdogRuntime> {
    let Some(profile) = platform::WATCHDOG_PROFILE else {
        logging::info(
            logging::BOOT_SUBSYSTEM,
            format_args!("[WATCHDOG] No target watchdog profile; runtime disabled"),
        );
        return None;
    };
    let Some(backend) = board.take_watchdog() else {
        logging::error(
            logging::BOOT_SUBSYSTEM,
            format_args!("[WATCHDOG] Backend unavailable; runtime disabled"),
        );
        return None;
    };
    match WatchdogRuntime::new(backend, profile) {
        Ok(runtime) => Some(runtime),
        Err(error) => {
            logging::error(
                logging::BOOT_SUBSYSTEM,
                format_args!("[WATCHDOG] Invalid target contract: {:?}", error),
            );
            None
        }
    }
}

fn initialize_logging() {
    logging::initialize();
}

fn emit_boot_banner() {
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
