//! Bounded SDIO recovery and reinitialization policy.

use crate::{
    drivers::{Block, BlockReader, StorageError, StorageLifecycleControl},
    logging, platform,
};

/// Defines the `REINITIALIZATION_ATTEMPTS` bound used by this subsystem.
const REINITIALIZATION_ATTEMPTS: u32 = crate::drivers::lifecycle::policy::REINITIALIZATION_ATTEMPTS;
/// Defines the `REINITIALIZATION_DELAY_MS` bound used by this subsystem.
const REINITIALIZATION_DELAY_MS: u32 = crate::drivers::lifecycle::policy::REINITIALIZATION_DELAY_MS;
/// Defines the `BOOT_RETRY_LOG_INTERVAL` bound used by this subsystem.
pub(super) const BOOT_RETRY_LOG_INTERVAL: u32 = 1;
/// Defines the `RUNTIME_PROBE_LOG_INTERVAL` bound used by this subsystem.
const RUNTIME_PROBE_LOG_INTERVAL: u32 = 10;

const _: () = assert!(BOOT_RETRY_LOG_INTERVAL > 0 && RUNTIME_PROBE_LOG_INTERVAL > 0);

/// Performs the `delay_before_reinitialization` operation for this subsystem.
fn delay_before_reinitialization<B>(board: &mut platform::Platform<B>)
where
    B: dali_kernel_api::BoardBackend,
    B::Watchdog: dali_kernel_api::WatchdogBackend,
{
    let _ = board.delay_ms(REINITIALIZATION_DELAY_MS);
    if let Err(error) = board.service_watchdog() {
        logging::error(
            logging::BOOT_SUBSYSTEM,
            format_args!(
                "[WATCHDOG] Feed failed during storage recovery: {:?}",
                error
            ),
        );
    }
}

/// Attempts initialization again for a bounded card-reinsert window.
pub(super) fn initialize_with_recovery<R, B>(
    reader: &mut R,
    board: &mut platform::Platform<B>,
    retry_log_interval: u32,
    retry_log_count: &mut u32,
) -> Result<(), StorageError>
where
    R: StorageLifecycleControl,
    B: dali_kernel_api::BoardBackend,
    B::Watchdog: dali_kernel_api::WatchdogBackend,
{
    let mut attempt = 0;
    loop {
        #[cfg(feature = "driver-hardware-test")]
        board.poll_user_key();
        let result = if attempt == 0 {
            reader.initialize()
        } else {
            reader.reinitialize()
        };
        match result {
            Ok(()) => return Ok(()),
            Err(StorageError::CardRemoved) if attempt + 1 < REINITIALIZATION_ATTEMPTS => {
                attempt += 1;
                *retry_log_count = retry_log_count.saturating_add(1);
                if (*retry_log_count).is_multiple_of(retry_log_interval) {
                    logging::info(
                        logging::BOOT_SUBSYSTEM,
                        format_args!(
                            "[STORAGE] Card unavailable; bounded reinitialization attempt {}/{}",
                            attempt + 1,
                            REINITIALIZATION_ATTEMPTS
                        ),
                    );
                }
                delay_before_reinitialization(board);
            }
            Err(error) => return Err(error),
        }
    }
}

/// Retries the first block read once after a bounded card reinitialization.
pub(super) fn read_block_with_recovery<R, B>(
    reader: &mut R,
    board: &mut platform::Platform<B>,
    block: &mut Block,
    retry_log_count: &mut u32,
) -> Result<(), StorageError>
where
    R: BlockReader + StorageLifecycleControl,
    B: dali_kernel_api::BoardBackend,
    B::Watchdog: dali_kernel_api::WatchdogBackend,
{
    match reader.read_block(crate::drivers::BlockAddress::new(0), block) {
        Ok(()) => Ok(()),
        Err(StorageError::CardRemoved) => {
            logging::info(
                logging::BOOT_SUBSYSTEM,
                format_args!("[STORAGE] Card removal detected; starting bounded reinitialization"),
            );
            initialize_with_recovery(reader, board, BOOT_RETRY_LOG_INTERVAL, retry_log_count)?;
            reader.read_block(crate::drivers::BlockAddress::new(0), block)
        }
        Err(error) => Err(error),
    }
}

/// Performs the `poll_runtime` operation for this subsystem.
pub(super) fn poll_runtime<B>(
    runtime: &mut super::super::StorageRuntime<B>,
    board: &mut platform::Platform<B>,
) where
    B: dali_kernel_api::BoardBackend,
    B::Watchdog: dali_kernel_api::WatchdogBackend,
{
    let Some(mut reader) = runtime.recovery_reader.take() else {
        return;
    };
    runtime.recovery_probe_count = runtime.recovery_probe_count.saturating_add(1);
    if matches!(
        runtime.status,
        super::super::lifecycle::status::StorageStatus::Removed
    ) && runtime
        .recovery_probe_count
        .is_multiple_of(RUNTIME_PROBE_LOG_INTERVAL)
    {
        logging::info(
            logging::BOOT_SUBSYSTEM,
            format_args!("[STORAGE] Reinitialization probe started"),
        );
    }
    match reader.reinitialize() {
        Ok(()) => {
            *runtime = super::initialization::load_initialized_reader(reader, board);
        }
        Err(StorageError::CardRemoved | StorageError::NotReady) => {
            runtime.recovery_reader = Some(reader);
            runtime.status = super::super::lifecycle::status::StorageStatus::Removed;
        }
        Err(StorageError::Timeout | StorageError::Transport) => {
            runtime.recovery_reader = Some(reader);
            runtime.status = super::super::lifecycle::status::StorageStatus::Idle;
        }
        Err(error) => {
            runtime.recovery_reader = Some(reader);
            runtime.status = super::super::lifecycle::status::StorageStatus::Failure;
            logging::error(
                logging::BOOT_SUBSYSTEM,
                format_args!("[STORAGE] Recovery reinitialization failed: {:?}", error),
            );
        }
    }
}
