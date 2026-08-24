//! Package validation, slot activation, and application launch policy.

use super::super::lifecycle::status;
use crate::{drivers::StorageError, logging, platform};

#[cfg(feature = "sdio")]
/// Performs the `load` operation for this subsystem.
/// Performs the `load` operation for this subsystem.
pub fn load<D>(
    device: D,
    board: &mut platform::Platform,
    committed_generation: Option<crate::storage::durable::coordinator::DurableGeneration>,
    #[cfg(feature = "abi-current")] slot_manager: &mut crate::runtime::memory::slots::SlotManager,
    #[cfg(feature = "abi-mpu")]
    context_owner: &mut crate::runtime::application::owner::ActiveContextOwner,
) -> status::StorageStatus
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    #[cfg(not(feature = "abi-context-switch"))]
    let _ = board;
    #[cfg(not(all(feature = "abi-current", feature = "repository-loader")))]
    let _ = committed_generation;

    logging::info(
        logging::BOOT_SUBSYSTEM,
        format_args!("[STORAGE] Read block 0 successfully"),
    );
    #[cfg(all(feature = "abi-current", feature = "repository-loader"))]
    let package =
        crate::loader::load_repository_package(device, slot_manager, committed_generation);
    #[cfg(all(feature = "abi-current", not(feature = "repository-loader")))]
    let package = crate::loader::load_current_abi(device, slot_manager);
    #[cfg(not(feature = "abi-current"))]
    let package = if platform::APPLICATION_EXECUTION_SUPPORTED {
        crate::loader::load_amrn_file(device)
    } else {
        crate::loader::validate_amrn_file(device)
    };

    match package {
        Ok(package) => {
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
            if platform::APPLICATION_EXECUTION_SUPPORTED {
                crate::loader::start_application(package);
            }
            #[cfg(feature = "abi-current")]
            {
                logging::info(
                    logging::BOOT_SUBSYSTEM,
                    format_args!(
                        "[LOADER] Loaded {} application package(s) into declared slots",
                        package.len()
                    ),
                );
                for application in package.iter() {
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
                    package
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
                    let Some(package) = package.first() else {
                        logging::error(
                            logging::SECURITY_SUBSYSTEM,
                            format_args!("[SECURITY] No loaded application context"),
                        );
                        return status::StorageStatus::Failure;
                    };
                    let Some(mut lifecycle) = package.lifecycle else {
                        if platform::activate_application_regions(package.slot) {
                            crate::security::launch::enter(package.launch_frame);
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
                            let Some(profile) = platform::SCHEDULER_PROFILE else {
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
                        }
                        crate::security::launch::enter(package.launch_frame);
                    }
                    logging::error(
                        logging::SECURITY_SUBSYSTEM,
                        format_args!("[SECURITY] Application MPU layout unavailable"),
                    );
                    status::StorageStatus::Failure
                }
                #[cfg(not(feature = "abi-mpu"))]
                {
                    let _ = package.first();
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
                    format_args!("[LOADER] No AMRN package found; entering kernel heartbeat"),
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
