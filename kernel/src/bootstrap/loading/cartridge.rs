//! Cartridge validation, slot activation, and application launch policy.

#[cfg(feature = "sdio")]
use super::super::lifecycle::status;
#[cfg(feature = "sdio")]
use crate::{drivers::StorageError, logging, platform};

#[cfg(feature = "sdio")]
/// Performs the `load` operation for this subsystem.
/// Performs the `load` operation for this subsystem.
pub fn load<D, B>(
    device: D,
    board: &mut platform::Platform<B>,
    committed_generation: Option<crate::storage::durable::coordinator::DurableGeneration>,
    #[cfg(feature = "abi-current")] slot_manager: &mut crate::runtime::memory::slots::SlotManager,
    #[cfg(feature = "abi-mpu")]
    context_owner: &mut crate::runtime::application::owner::ActiveContextOwner,
) -> status::StorageStatus
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
    B: dali_kernel_api::BoardBackend,
    B::Watchdog: dali_kernel_api::WatchdogBackend,
{
    #[cfg(not(feature = "abi-context-switch"))]
    let _ = board;
    #[cfg(not(all(feature = "abi-current", feature = "repository-loader")))]
    let _ = committed_generation;

    logging::info(
        logging::BOOT_SUBSYSTEM,
        format_args!("[STORAGE] Read block 0 successfully"),
    );
    #[cfg(all(feature = "artifact-flash", feature = "abi-current"))]
    let cartridge = match board.take_artifact_reader() {
        Some(reader) => match crate::loader::load_flash_cartridge(reader, slot_manager) {
            Ok(cartridge) => Ok(cartridge),
            Err(crate::loader::LoaderError::Filesystem(embedded_sdmmc::Error::NotFound)) => {
                load_from_sdio::<D, B>(device, slot_manager, committed_generation)
            }
            Err(error) => Err(error),
        },
        None => load_from_sdio::<D, B>(device, slot_manager, committed_generation),
    };
    #[cfg(all(
        feature = "abi-current",
        not(feature = "artifact-flash"),
        feature = "repository-loader"
    ))]
    let cartridge =
        crate::loader::load_repository_cartridge(device, slot_manager, committed_generation);
    #[cfg(all(
        feature = "abi-current",
        not(feature = "artifact-flash"),
        not(feature = "repository-loader")
    ))]
    let cartridge = crate::loader::load_current_abi::<D, B>(device, slot_manager);
    #[cfg(not(feature = "abi-current"))]
    let cartridge = if platform::Platform::<B>::supports_application_execution() {
        crate::loader::load_amrn_file(device)
    } else {
        crate::loader::validate_amrn_file(device)
    };

    match cartridge {
        Ok(cartridge) => {
            logging::info(
                logging::BOOT_SUBSYSTEM,
                format_args!("[LOADER] AMRN header and payload validated"),
            );
            #[cfg(feature = "abi-authentication")]
            logging::info(
                logging::SECURITY_SUBSYSTEM,
                format_args!("[SECURITY] AMRN signature verified"),
            );
            #[cfg(not(feature = "abi-current"))]
            if platform::Platform::<B>::supports_application_execution() {
                crate::loader::start_application(cartridge);
            }
            #[cfg(feature = "abi-current")]
            {
                logging::info(
                    logging::BOOT_SUBSYSTEM,
                    format_args!(
                        "[LOADER] Loaded {} application cartridge(s) into declared slots",
                        cartridge.len()
                    ),
                );
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
                {
                    let Some(cartridge) = cartridge.first() else {
                        logging::error(
                            logging::SECURITY_SUBSYSTEM,
                            format_args!("[SECURITY] No loaded application context"),
                        );
                        return status::StorageStatus::Failure;
                    };
                    let Some(mut lifecycle) = cartridge.lifecycle else {
                        if platform::activate_application_regions(cartridge.slot) {
                            crate::security::launch::enter(cartridge.launch_frame);
                        }
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
                    if platform::activate_application_regions(active.allocation().slot()) {
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
                            // SysTick must not run until the scheduler and the
                            // first application context are fully initialized.
                            platform::Platform::<B>::enable_interrupts();
                        }
                        crate::security::launch::enter(cartridge.launch_frame);
                    }
                    logging::error(
                        logging::SECURITY_SUBSYSTEM,
                        format_args!("[SECURITY] Application MPU layout unavailable"),
                    );
                    status::StorageStatus::Failure
                }
                #[cfg(not(feature = "abi-mpu"))]
                {
                    let _ = cartridge.first();
                    status::StorageStatus::Ready
                }
            }
            #[cfg(not(feature = "abi-current"))]
            status::StorageStatus::Ready
        }
        Err(error) => {
            if matches!(
                error,
                crate::loader::LoaderError::Filesystem(embedded_sdmmc::Error::NotFound)
            ) {
                logging::info(
                    logging::BOOT_SUBSYSTEM,
                    format_args!("[LOADER] No AMRN cartridge found; entering kernel heartbeat"),
                );
                return status::StorageStatus::Idle;
            }
            logging::error(
                logging::BOOT_SUBSYSTEM,
                format_args!("[LOADER] AMRN validation failed: {:?}", error),
            );
            status::StorageStatus::Failure
        }
    }
}

#[cfg(all(feature = "sdio", feature = "abi-current", feature = "artifact-flash"))]
/// Preserves the existing SD repository and root loading paths as fallback.
fn load_from_sdio<D, B>(
    device: D,
    slot_manager: &mut crate::runtime::memory::slots::SlotManager,
    committed_generation: Option<crate::storage::durable::coordinator::DurableGeneration>,
) -> Result<crate::loader::LoadedCartridges, crate::loader::LoaderError>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
    B: dali_kernel_api::BoardBackend,
    B::Watchdog: dali_kernel_api::WatchdogBackend,
{
    #[cfg(feature = "repository-loader")]
    {
        crate::loader::load_repository_cartridge(device, slot_manager, committed_generation)
    }
    #[cfg(not(feature = "repository-loader"))]
    {
        let _ = committed_generation;
        crate::loader::load_current_abi::<D, B>(device, slot_manager)
    }
}
