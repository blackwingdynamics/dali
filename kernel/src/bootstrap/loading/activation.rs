//! Shared application activation after a source-specific load succeeds.

use super::super::lifecycle::status;
use crate::{logging, platform};

#[cfg(feature = "abi-current")]
/// Registers, protects, and launches the loaded application set.
pub(super) fn activate<B>(
    cartridge: crate::loader::LoadedCartridges,
    #[cfg(any(feature = "abi-mpu", feature = "abi-context-switch"))] board: &mut platform::Platform<
        B,
    >,
    #[cfg(feature = "abi-mpu")]
    context_owner: &mut crate::runtime::application::owner::ActiveContextOwner,
) -> status::StorageStatus
where
    B: dali_kernel_api::BoardBackend,
    B::Watchdog: dali_kernel_api::WatchdogBackend,
{
    logging::info(
        logging::BOOT_SUBSYSTEM,
        format_args!(
            "[LOADER] Loaded {} application cartridge(s) into declared slots",
            cartridge.len()
        ),
    );
    #[cfg(all(feature = "abi-mpu", not(feature = "abi-context-switch")))]
    let _ = board;
    for application in cartridge.iter() {
        let slot = application.slot;
        logging::info(
            logging::BOOT_SUBSYSTEM,
            format_args!(
                "[LOADER] Slot {} ({}) boundaries: code=0x{:08X}+{} data=0x{:08X}+{} psp_top=0x{:08X}",
                slot.id,
                slot.name,
                slot.code_origin,
                slot.code_length,
                slot.data_origin,
                slot.data_length,
                application.psp_top,
            ),
        );
    }
    #[cfg(feature = "abi-context-switch")]
    if let Err(error) = crate::security::scheduling::register_contexts(
        cartridge
            .iter()
            .map(|application| application.scheduler_context()),
    ) {
        logging::error(
            logging::SECURITY_SUBSYSTEM,
            format_args!(
                "[SECURITY] Scheduler context registration failed: {:?}",
                error
            ),
        );
        return status::StorageStatus::Failure;
    }
    #[cfg(feature = "abi-mpu")]
    return activate_first::<B>(
        cartridge,
        #[cfg(feature = "abi-context-switch")]
        board,
        context_owner,
    );
    #[cfg(not(feature = "abi-mpu"))]
    {
        let _ = (cartridge, board);
        status::StorageStatus::Ready
    }
}

#[cfg(feature = "abi-mpu")]
/// Performs the lifecycle and MPU transition for the first loaded context.
fn activate_first<B>(
    cartridge: crate::loader::LoadedCartridges,
    #[cfg(feature = "abi-context-switch")] board: &mut platform::Platform<B>,
    context_owner: &mut crate::runtime::application::owner::ActiveContextOwner,
) -> status::StorageStatus
where
    B: dali_kernel_api::BoardBackend,
    B::Watchdog: dali_kernel_api::WatchdogBackend,
{
    let Some(cartridge) = cartridge.first() else {
        logging::error(
            logging::SECURITY_SUBSYSTEM,
            format_args!("[SECURITY] No loaded application context"),
        );
        return status::StorageStatus::Failure;
    };
    let Some(mut lifecycle) = cartridge.lifecycle else {
        logging::error(
            logging::SECURITY_SUBSYSTEM,
            format_args!("[SECURITY] Application lifecycle unavailable"),
        );
        return status::StorageStatus::Failure;
    };
    if lifecycle
        .transition(crate::runtime::application::lifecycle::LifecycleEvent::Ready)
        .is_err()
    {
        logging::error(
            logging::SECURITY_SUBSYSTEM,
            format_args!("[SECURITY] Application lifecycle not load-ready"),
        );
        return status::StorageStatus::Failure;
    }
    logging::info(
        logging::SECURITY_SUBSYSTEM,
        format_args!("[SECURITY] Application lifecycle: Ready"),
    );
    if let Err(error) = context_owner.activate(&mut lifecycle) {
        logging::error(
            logging::SECURITY_SUBSYSTEM,
            format_args!(
                "[SECURITY] Application context activation failed: {:?}",
                error
            ),
        );
        return status::StorageStatus::Failure;
    }
    logging::info(
        logging::SECURITY_SUBSYSTEM,
        format_args!("[SECURITY] Active application context: Running"),
    );
    #[cfg(feature = "abi-context-switch")]
    if let Err(error) = crate::security::scheduling::activate_first() {
        logging::error(
            logging::SECURITY_SUBSYSTEM,
            format_args!(
                "[SECURITY] Scheduler context activation failed: {:?}",
                error
            ),
        );
        return status::StorageStatus::Failure;
    }
    let Some(active) = context_owner.active() else {
        logging::error(
            logging::SECURITY_SUBSYSTEM,
            format_args!("[SECURITY] Active application context unavailable"),
        );
        return status::StorageStatus::Failure;
    };
    if !platform::activate_application_regions(active.allocation().slot()) {
        logging::error(
            logging::SECURITY_SUBSYSTEM,
            format_args!("[SECURITY] Application MPU layout unavailable"),
        );
        return status::StorageStatus::Failure;
    }
    #[cfg(feature = "abi-context-switch")]
    {
        let Some(profile) = platform::scheduler_profile() else {
            logging::error(
                logging::SECURITY_SUBSYSTEM,
                format_args!("[SECURITY] Scheduler profile unavailable"),
            );
            return status::StorageStatus::Failure;
        };
        if !board.enable_scheduler_tick(profile.tick_hz) {
            logging::error(
                logging::SECURITY_SUBSYSTEM,
                format_args!("[SECURITY] Scheduler tick configuration invalid"),
            );
            return status::StorageStatus::Failure;
        }
        platform::Platform::<B>::enable_interrupts();
    }
    crate::security::launch::enter(cartridge.launch_frame);
}
