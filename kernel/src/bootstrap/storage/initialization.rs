//! Storage initialization and AMRN loading policy.

use super::super::lifecycle::status;
use crate::{
    drivers::{BLOCK_SIZE, Block, BlockAddress, BlockReader, StorageError},
    logging, platform,
};

#[cfg(not(feature = "storage-write"))]
use crate::drivers::BlockDeviceAdapter;

#[cfg(feature = "sdio")]
pub fn initialize(
    board: &mut platform::Platform,
    boot_mode: status::BootMode,
) -> status::StorageStatus {
    if boot_mode == status::BootMode::SafeMode {
        logging::info(
            logging::SECURITY_SUBSYSTEM,
            format_args!("[RECOVERY] Safe Mode active; application loading skipped"),
        );
        return status::StorageStatus::SafeMode;
    }

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
        Ok(()) => {
            #[cfg(feature = "storage-write")]
            {
                let device = crate::drivers::WritableBlockDeviceAdapter::new(reader);
                if let Err(error) = super::acceptance::verify_trust_store_artifacts(&device) {
                    logging::error(
                        logging::BOOT_SUBSYSTEM,
                        format_args!("[STORAGE] Trust-store artifact test failed: {:?}", error),
                    );
                    return status::StorageStatus::Failure;
                }
                logging::info(
                    logging::BOOT_SUBSYSTEM,
                    format_args!("[STORAGE] Trust-store artifact write/read-back test passed"),
                );
                super::super::loading::load(
                    &device,
                    board,
                    #[cfg(feature = "abi-current")]
                    &mut slot_manager,
                    #[cfg(feature = "abi-mpu")]
                    &mut context_owner,
                )
            }
            #[cfg(not(feature = "storage-write"))]
            super::super::loading::load(
                BlockDeviceAdapter::new(reader),
                board,
                #[cfg(feature = "abi-current")]
                &mut slot_manager,
                #[cfg(feature = "abi-mpu")]
                &mut context_owner,
            )
        }
        Err(error) => {
            logging::error(
                logging::BOOT_SUBSYSTEM,
                format_args!("[STORAGE] Block 0 read failed: {:?}", error),
            );
            status::StorageStatus::Failure
        }
    }
}

#[cfg(not(feature = "sdio"))]
pub fn initialize(
    _board: &mut platform::Platform,
    _boot_mode: status::BootMode,
) -> status::StorageStatus {
    status::StorageStatus::NotDetected
}
