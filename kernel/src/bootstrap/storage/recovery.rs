//! Bounded SDIO recovery and reinitialization policy.

use crate::{
    drivers::{Block, BlockReader, StorageError, StorageLifecycleControl},
    logging, platform,
};

const REINITIALIZATION_ATTEMPTS: u32 = crate::drivers::lifecycle::policy::REINITIALIZATION_ATTEMPTS;
const REINITIALIZATION_DELAY_MS: u32 = crate::drivers::lifecycle::policy::REINITIALIZATION_DELAY_MS;
pub(super) const BOOT_RETRY_LOG_INTERVAL: u32 = 1;
#[cfg(feature = "driver-hardware-test")]
const RUNTIME_RETRY_LOG_INTERVAL: u32 = 50;
#[cfg(not(feature = "driver-hardware-test"))]
const RUNTIME_RETRY_LOG_INTERVAL: u32 = 1;
const RUNTIME_PROBE_LOG_INTERVAL: u32 = 10;

const _: () = assert!(
    BOOT_RETRY_LOG_INTERVAL > 0 && RUNTIME_RETRY_LOG_INTERVAL > 0 && RUNTIME_PROBE_LOG_INTERVAL > 0
);

fn delay_before_reinitialization(board: &mut platform::Platform) {
    board.delay_ms(REINITIALIZATION_DELAY_MS);
    if let Err(error) = platform::service_watchdog() {
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
pub(super) fn initialize_with_recovery<R>(
    reader: &mut R,
    board: &mut platform::Platform,
    retry_log_interval: u32,
    retry_log_count: &mut u32,
) -> Result<(), StorageError>
where
    R: StorageLifecycleControl,
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
pub(super) fn read_block_with_recovery<R>(
    reader: &mut R,
    board: &mut platform::Platform,
    block: &mut Block,
    retry_log_count: &mut u32,
) -> Result<(), StorageError>
where
    R: BlockReader + StorageLifecycleControl,
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

pub(super) fn poll_runtime(
    runtime: &mut super::super::StorageRuntime,
    board: &mut platform::Platform,
) {
    let Some(reader) = runtime.recovery_reader.as_mut() else {
        return;
    };
    runtime.recovery_probe_count = runtime.recovery_probe_count.saturating_add(1);
    if runtime
        .recovery_probe_count
        .is_multiple_of(RUNTIME_PROBE_LOG_INTERVAL)
    {
        logging::info(
            logging::BOOT_SUBSYSTEM,
            format_args!("[STORAGE] Reinitialization probe started"),
        );
    }
    match initialize_with_recovery(
        reader,
        board,
        RUNTIME_RETRY_LOG_INTERVAL,
        &mut runtime.recovery_retry_log_count,
    ) {
        Ok(()) => {
            runtime.status = super::super::lifecycle::status::StorageStatus::Ready;
            logging::info(
                logging::BOOT_SUBSYSTEM,
                format_args!("[STORAGE] Card reinitialized; state Ready"),
            );
        }
        Err(
            StorageError::CardRemoved
            | StorageError::NotReady
            | StorageError::Timeout
            | StorageError::Transport,
        ) => {
            runtime.status = super::super::lifecycle::status::StorageStatus::Removed;
        }
        Err(error) => {
            runtime.status = super::super::lifecycle::status::StorageStatus::Failure;
            logging::error(
                logging::BOOT_SUBSYSTEM,
                format_args!("[STORAGE] Recovery reinitialization failed: {:?}", error),
            );
        }
    }
}
