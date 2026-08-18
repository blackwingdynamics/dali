//! Storage initialization and AMRN loading policy.

use super::status;
use crate::{
    drivers::{BLOCK_SIZE, Block, BlockAddress, BlockDeviceAdapter, BlockReader, StorageError},
    logging, platform,
};

#[cfg(feature = "sdio")]
pub(super) fn initialize(board: &mut platform::Platform) -> status::StorageStatus {
    let Some(mut reader) = board.take_sdio_reader() else {
        logging::error(
            logging::BOOT_SUBSYSTEM,
            format_args!("[STORAGE] Storage interface unavailable"),
        );
        return status::StorageStatus::Failure;
    };

    if let Err(error) = reader.initialize() {
        return match error {
            StorageError::NotReady | StorageError::Timeout => {
                logging::info(
                    logging::BOOT_SUBSYSTEM,
                    format_args!("[STORAGE] No storage medium detected; entering kernel heartbeat"),
                );
                status::StorageStatus::NotDetected
            }
            error => {
                logging::error(
                    logging::BOOT_SUBSYSTEM,
                    format_args!("[STORAGE] Storage initialization failed: {:?}", error),
                );
                status::StorageStatus::Failure
            }
        };
    }

    logging::info(
        logging::BOOT_SUBSYSTEM,
        format_args!("[STORAGE] SDIO card initialized"),
    );

    #[cfg(feature = "abi-current")]
    let mut slot_manager = match crate::runtime::memory::slots::SlotManager::new(
        platform::MEMORY_PROFILE
            .isolation
            .map(|isolation| isolation.slots)
            .unwrap_or(&[]),
    ) {
        Ok(manager) => manager,
        Err(error) => {
            logging::error(
                logging::SECURITY_SUBSYSTEM,
                format_args!("[SECURITY] Slot manager unavailable: {:?}", error),
            );
            return status::StorageStatus::Failure;
        }
    };
    #[cfg(feature = "abi-mpu")]
    let mut context_owner = crate::runtime::application::owner::ActiveContextOwner::new(
        &crate::runtime::application::owner::ACTIVE_RUNTIME_STATE,
    );

    let mut block: Block = [0; BLOCK_SIZE];
    match reader.read_block(BlockAddress::new(0), &mut block) {
        Ok(()) => load_package(
            reader,
            #[cfg(feature = "abi-current")]
            &mut slot_manager,
            #[cfg(feature = "abi-mpu")]
            &mut context_owner,
        ),
        Err(error) => {
            logging::error(
                logging::BOOT_SUBSYSTEM,
                format_args!("[STORAGE] Block 0 read failed: {:?}", error),
            );
            status::StorageStatus::Failure
        }
    }
}

#[cfg(feature = "sdio")]
fn load_package<R>(
    reader: R,
    #[cfg(feature = "abi-current")] slot_manager: &mut crate::runtime::memory::slots::SlotManager,
    #[cfg(feature = "abi-mpu")]
    context_owner: &mut crate::runtime::application::owner::ActiveContextOwner,
) -> status::StorageStatus
where
    R: BlockReader,
{
    logging::info(
        logging::BOOT_SUBSYSTEM,
        format_args!("[STORAGE] Read block 0 successfully"),
    );
    #[cfg(feature = "abi-current")]
    let package = crate::loader::load_current_abi(BlockDeviceAdapter::new(reader), slot_manager);
    #[cfg(not(feature = "abi-current"))]
    let package = if platform::APPLICATION_EXECUTION_SUPPORTED {
        crate::loader::load_amrn_file(BlockDeviceAdapter::new(reader))
    } else {
        crate::loader::validate_amrn_file(BlockDeviceAdapter::new(reader))
    };

    match package {
        Ok(package) => {
            logging::info(
                logging::BOOT_SUBSYSTEM,
                format_args!("[LOADER] AMRN header and payload validated"),
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

#[cfg(not(feature = "sdio"))]
pub(super) fn initialize(_board: &mut platform::Platform) -> status::StorageStatus {
    status::StorageStatus::NotDetected
}
