//! Watchdog backend initialization during kernel startup.

use crate::runtime::watchdog::WatchdogRuntime;
use crate::{logging, platform};

pub fn initialize(board: &mut platform::Platform) -> Option<platform::WatchdogRuntime> {
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
