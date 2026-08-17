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
            format_args!("[STORAGE] SDIO resources unavailable"),
        );
        return status::StorageStatus::Failure;
    };

    if let Err(error) = reader.initialize() {
        logging::error(
            logging::BOOT_SUBSYSTEM,
            format_args!("[STORAGE] SDIO initialization failed: {:?}", error),
        );
        return match error {
            StorageError::NotReady | StorageError::Timeout => status::StorageStatus::NotDetected,
            _ => status::StorageStatus::Failure,
        };
    }

    logging::info(
        logging::BOOT_SUBSYSTEM,
        format_args!("[STORAGE] SDIO card initialized"),
    );

    let mut block: Block = [0; BLOCK_SIZE];
    match reader.read_block(BlockAddress::new(0), &mut block) {
        Ok(()) => load_package(reader),
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
fn load_package<R>(reader: R) -> status::StorageStatus
where
    R: BlockReader,
{
    logging::info(
        logging::BOOT_SUBSYSTEM,
        format_args!("[STORAGE] Read block 0 successfully"),
    );
    #[cfg(feature = "abi-current")]
    let package = crate::loader::load_abi_v3(BlockDeviceAdapter::new(reader));
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
                #[cfg(feature = "abi-mpu")]
                {
                    if platform::activate_application_regions() {
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
                    let _ = package;
                    status::StorageStatus::Ready
                }
            }
            #[cfg(not(feature = "abi-current"))]
            status::StorageStatus::Ready
        }
        Err(error) => {
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
