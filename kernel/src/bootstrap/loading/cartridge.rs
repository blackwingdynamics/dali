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
            return super::activation::activate(cartridge, board, context_owner);
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

#[cfg(all(feature = "artifact-flash", feature = "abi-current"))]
/// Attempts to boot one signed cartridge before initializing removable storage.
pub(crate) fn try_load_flash<B>(board: &mut platform::Platform<B>) -> Option<status::StorageStatus>
where
    B: dali_kernel_api::BoardBackend,
    B::Watchdog: dali_kernel_api::WatchdogBackend,
{
    let reader = board.take_artifact_reader()?;
    let mut slot_manager = match crate::runtime::memory::slots::SlotManager::new(
        platform::memory_profile()
            .and_then(|memory| memory.isolation)
            .map(|isolation| isolation.slots)
            .unwrap_or(&[]),
    ) {
        Ok(manager) => manager,
        Err(error) => {
            logging::error(
                logging::SECURITY_SUBSYSTEM,
                format_args!("[SECURITY] Slot manager unavailable: {:?}", error),
            );
            return Some(status::StorageStatus::Failure);
        }
    };
    #[cfg(feature = "abi-mpu")]
    let mut context_owner = crate::runtime::application::owner::ActiveContextOwner::new(
        &crate::runtime::application::owner::ACTIVE_RUNTIME_STATE,
    );
    let cartridge = crate::loader::load_flash_cartridge(reader, &mut slot_manager);
    match cartridge {
        Ok(cartridge) => {
            logging::info(
                logging::BOOT_SUBSYSTEM,
                format_args!("[STORAGE] Booting signed AMRN from artifact Flash"),
            );
            logging::info(
                logging::SECURITY_SUBSYSTEM,
                format_args!("[SECURITY] AMRN signature verified"),
            );
            Some(super::activation::activate(
                cartridge,
                board,
                #[cfg(feature = "abi-mpu")]
                &mut context_owner,
            ))
        }
        Err(crate::loader::LoaderError::Filesystem(embedded_sdmmc::Error::NotFound)) => {
            logging::info(
                logging::BOOT_SUBSYSTEM,
                format_args!("[STORAGE] Artifact Flash region empty; falling back to SD"),
            );
            None
        }
        Err(error) => {
            logging::error(
                logging::BOOT_SUBSYSTEM,
                format_args!("[LOADER] Flash AMRN validation failed: {:?}", error),
            );
            Some(status::StorageStatus::Failure)
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
