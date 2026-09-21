//! Storage initialization and AMRN loading policy.

#[cfg(feature = "sdio")]
use super::super::StorageRuntime;
use super::super::lifecycle::status;
use crate::platform;
#[cfg(feature = "sdio")]
use crate::{
    drivers::{BLOCK_SIZE, Block, StorageError},
    logging,
};

#[cfg(feature = "sdio")]
use super::recovery::{
    BOOT_RETRY_LOG_INTERVAL, initialize_with_recovery, read_block_with_recovery,
};

#[cfg(all(feature = "sdio", not(feature = "storage-write")))]
use crate::drivers::BlockDeviceAdapter;

#[cfg(feature = "sdio")]
/// Performs the `initialize` operation for this subsystem.
pub fn initialize<B>(
    board: &mut platform::Platform<B>,
    boot_mode: status::BootMode,
) -> StorageRuntime<B>
where
    B: dali_kernel_api::BoardBackend,
    B::Watchdog: dali_kernel_api::WatchdogBackend,
{
    // Keep the watchdog service available in Safe Mode so recovery remains
    // stable after a watchdog reset instead of entering another reset loop.
    super::super::startup::install_watchdog(board);
    if boot_mode == status::BootMode::SafeMode {
        logging::info(
            logging::SECURITY_SUBSYSTEM,
            format_args!("[RECOVERY] Safe Mode active; application loading skipped"),
        );
        return StorageRuntime::from_status(status::StorageStatus::SafeMode);
    }

    #[cfg(all(feature = "artifact-flash", feature = "abi-current"))]
    if let Some(status) = super::super::loading::try_load_flash(board) {
        logging::info(
            logging::BOOT_SUBSYSTEM,
            format_args!("[STORAGE] Artifact Flash boot attempt completed"),
        );
        return StorageRuntime::from_status(status);
    }

    let Some(mut reader) = board.take_sdio_reader() else {
        logging::error(
            logging::BOOT_SUBSYSTEM,
            format_args!("[STORAGE] Storage interface unavailable"),
        );
        return StorageRuntime::from_status(status::StorageStatus::Failure);
    };

    let mut retry_log_count = 0;
    let initialization = initialize_with_recovery(
        &mut reader,
        board,
        BOOT_RETRY_LOG_INTERVAL,
        &mut retry_log_count,
    );
    if let Err(error) = initialization {
        return match error {
            StorageError::NotReady | StorageError::Timeout | StorageError::Transport => {
                logging::info(
                    logging::BOOT_SUBSYSTEM,
                    format_args!("[STORAGE] No SD card available; continuing without cartridge"),
                );
                StorageRuntime::with_recovery_reader(status::StorageStatus::Idle, reader)
            }
            StorageError::CardRemoved => {
                logging::info(
                    logging::BOOT_SUBSYSTEM,
                    format_args!("[STORAGE] No SD card available; continuing without cartridge"),
                );
                StorageRuntime::with_recovery_reader(status::StorageStatus::Idle, reader)
            }
            error => {
                logging::error(
                    logging::BOOT_SUBSYSTEM,
                    format_args!("[STORAGE] Storage initialization failed: {:?}", error),
                );
                StorageRuntime::from_status(status::StorageStatus::Failure)
            }
        };
    }

    load_initialized_reader(reader, board)
}

#[cfg(feature = "sdio")]
/// Loads a cartridge from a reader that has completed SDIO initialization.
pub(super) fn load_initialized_reader<B>(
    mut reader: B::StorageReader,
    board: &mut platform::Platform<B>,
) -> StorageRuntime<B>
where
    B: dali_kernel_api::BoardBackend,
    B::Watchdog: dali_kernel_api::WatchdogBackend,
{
    logging::info(
        logging::BOOT_SUBSYSTEM,
        format_args!("[STORAGE] SDIO card initialized"),
    );

    #[cfg(feature = "abi-current")]
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
            return StorageRuntime::from_status(status::StorageStatus::Failure);
        }
    };
    #[cfg(feature = "abi-mpu")]
    let mut context_owner = crate::runtime::application::owner::ActiveContextOwner::new(
        &crate::runtime::application::owner::ACTIVE_RUNTIME_STATE,
    );

    let block_read = {
        let mut block: Block = [0; BLOCK_SIZE];
        let mut retry_log_count = 0;
        read_block_with_recovery(&mut reader, board, &mut block, &mut retry_log_count)
    };
    match block_read {
        Ok(()) => {
            #[cfg(feature = "storage-write")]
            {
                let device = crate::drivers::WritableBlockDeviceAdapter::new(reader);
                let journal_recovery = match super::acceptance::recover_trust_store_journal(&device)
                {
                    Ok(super::acceptance::JournalRecovery::Missing) => {
                        logging::info(
                            logging::SECURITY_SUBSYSTEM,
                            format_args!(
                                "[RECOVERY] No durable commit journal found; using provisioned state"
                            ),
                        );
                        super::acceptance::JournalRecovery::Missing
                    }
                    Ok(super::acceptance::JournalRecovery::Decision(
                        crate::storage::durable::RecoveryDecision::Committed(record),
                    )) => {
                        logging::info(
                            logging::SECURITY_SUBSYSTEM,
                            format_args!(
                                "[RECOVERY] Committed generation selected: version={} sequence={} slot={:?}",
                                record.bundle_version, record.sequence, record.active_slot,
                            ),
                        );
                        super::acceptance::JournalRecovery::Decision(
                            crate::storage::durable::RecoveryDecision::Committed(record),
                        )
                    }
                    Ok(super::acceptance::JournalRecovery::Decision(
                        crate::storage::durable::RecoveryDecision::DiscardPrepared,
                    )) => {
                        logging::info(
                            logging::SECURITY_SUBSYSTEM,
                            format_args!(
                                "[RECOVERY] Prepared commit discarded; previous active state retained"
                            ),
                        );
                        super::acceptance::JournalRecovery::Decision(
                            crate::storage::durable::RecoveryDecision::DiscardPrepared,
                        )
                    }
                    Err(super::acceptance::ArtifactTestError::CommitJournal(
                        crate::storage::durable::JournalError::InvalidLength,
                    )) => {
                        logging::warn(
                            logging::SECURITY_SUBSYSTEM,
                            format_args!(
                                "[RECOVERY] Invalid commit journal length; rebuilding provisioned test state"
                            ),
                        );
                        super::acceptance::JournalRecovery::Missing
                    }
                    Err(error) => {
                        logging::error(
                            logging::SECURITY_SUBSYSTEM,
                            format_args!("[RECOVERY] Commit journal recovery failed: {:?}", error),
                        );
                        return StorageRuntime::from_status(status::StorageStatus::Failure);
                    }
                };
                let committed_generation = match journal_recovery {
                    super::acceptance::JournalRecovery::Decision(
                        crate::storage::durable::RecoveryDecision::Committed(record),
                    ) => Some(crate::storage::durable::coordinator::DurableGeneration {
                        version: record.bundle_version,
                        digest: record.bundle_digest,
                        length: record.bundle_length,
                    }),
                    _ => None,
                };
                #[cfg(not(feature = "storage-interruption-test"))]
                let _ = journal_recovery;
                #[cfg(feature = "storage-interruption-test")]
                if !matches!(
                    journal_recovery,
                    super::acceptance::JournalRecovery::Decision(
                        crate::storage::durable::RecoveryDecision::DiscardPrepared
                    )
                ) {
                    if let Err(error) = super::interruption::stage_prepared_journal(&device) {
                        logging::error(
                            logging::SECURITY_SUBSYSTEM,
                            format_args!(
                                "[ACCEPTANCE] Prepared interruption fixture failed: {:?}",
                                error
                            ),
                        );
                        return StorageRuntime::from_status(status::StorageStatus::Failure);
                    }
                    logging::info(
                        logging::SECURITY_SUBSYSTEM,
                        format_args!(
                            "[ACCEPTANCE] Prepared interruption journal staged; reset target to verify recovery"
                        ),
                    );
                    return StorageRuntime::from_status(status::StorageStatus::Failure);
                }
                if let Err(error) = super::acceptance::verify_trust_store_artifacts(&device) {
                    logging::error(
                        logging::BOOT_SUBSYSTEM,
                        format_args!("[STORAGE] Trust-store artifact test failed: {:?}", error),
                    );
                    return StorageRuntime::from_status(status::StorageStatus::Failure);
                }
                logging::info(
                    logging::BOOT_SUBSYSTEM,
                    format_args!(
                        "[STORAGE] Trust-store artifact write/flush/read-back test passed"
                    ),
                );
                let status = super::super::loading::load(
                    &device,
                    board,
                    committed_generation,
                    #[cfg(feature = "abi-current")]
                    &mut slot_manager,
                    #[cfg(feature = "abi-mpu")]
                    &mut context_owner,
                );
                logging::info(
                    logging::BOOT_SUBSYSTEM,
                    format_args!("[LOADER] Cartridge loading returned: {:?}", status),
                );
                StorageRuntime::from_status(status)
            }
            #[cfg(not(feature = "storage-write"))]
            {
                let status = super::super::loading::load(
                    BlockDeviceAdapter::new(reader),
                    board,
                    None,
                    #[cfg(feature = "abi-current")]
                    &mut slot_manager,
                    #[cfg(feature = "abi-mpu")]
                    &mut context_owner,
                );
                logging::info(
                    logging::BOOT_SUBSYSTEM,
                    format_args!("[LOADER] Cartridge loading returned: {:?}", status),
                );
                StorageRuntime::from_status(status)
            }
        }
        Err(StorageError::CardRemoved | StorageError::NotReady | StorageError::Transport) => {
            logging::info(
                logging::BOOT_SUBSYSTEM,
                format_args!("[STORAGE] No SD card available; continuing without cartridge"),
            );
            StorageRuntime::with_recovery_reader(status::StorageStatus::Idle, reader)
        }
        Err(error) => {
            logging::error(
                logging::BOOT_SUBSYSTEM,
                format_args!("[STORAGE] Block 0 read failed: {:?}", error),
            );
            StorageRuntime::from_status(status::StorageStatus::Failure)
        }
    }
}

#[cfg(not(feature = "sdio"))]
pub fn initialize<B>(
    _board: &mut platform::Platform<B>,
    _boot_mode: status::BootMode,
) -> super::StorageRuntime<B>
where
    B: dali_kernel_api::BoardBackend,
    B::Watchdog: dali_kernel_api::WatchdogBackend,
{
    super::StorageRuntime::from_status(status::StorageStatus::NotDetected)
}
