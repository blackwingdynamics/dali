//! Watchdog backend initialization during kernel startup.

use crate::runtime::watchdog::{FeedOwner, WatchdogRuntime};
use crate::{logging, platform};

/// Performs the `initialize` operation for this subsystem.
pub fn initialize<B>(board: &mut platform::Platform<B>) -> Option<WatchdogRuntime<B::Watchdog>>
where
    B: dali_kernel_api::BoardBackend,
    B::Watchdog: dali_kernel_api::WatchdogBackend,
{
    let profile = B::info().target.watchdog?;
    let Ok(backend) = board.take_watchdog() else {
        logging::error(
            logging::BOOT_SUBSYSTEM,
            format_args!("[WATCHDOG] Backend unavailable; runtime disabled"),
        );
        return None;
    };
    match WatchdogRuntime::new(backend, profile) {
        Ok(mut runtime) => match runtime.arm(FeedOwner::KernelHeartbeat) {
            Ok(()) => Some(runtime),
            Err(_) => {
                logging::error(
                    logging::BOOT_SUBSYSTEM,
                    format_args!("[WATCHDOG] Failed to arm runtime"),
                );
                None
            }
        },
        Err(error) => {
            logging::error(
                logging::BOOT_SUBSYSTEM,
                format_args!("[WATCHDOG] Invalid target contract: {:?}", error),
            );
            None
        }
    }
}

/// Installs the watchdog after the selected storage transport has completed
/// its opaque hardware initialization phase.
pub fn install<B>(board: &mut platform::Platform<B>)
where
    B: dali_kernel_api::BoardBackend,
    B::Watchdog: dali_kernel_api::WatchdogBackend,
{
    let Some(watchdog) = initialize(board) else {
        return;
    };
    if let Err(error) = board.install_watchdog(watchdog) {
        logging::error(
            logging::BOOT_SUBSYSTEM,
            format_args!("[WATCHDOG] Failed to install runtime: {:?}", error),
        );
    }
}
