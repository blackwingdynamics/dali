//! Bounded SDIO recovery and reinitialization policy.

use crate::{
    drivers::{Block, BlockReader, StorageError, StorageLifecycleControl},
    logging, platform,
};

const REINITIALIZATION_ATTEMPTS: u32 = crate::drivers::lifecycle::policy::REINITIALIZATION_ATTEMPTS;
const REINITIALIZATION_DELAY_MS: u32 = crate::drivers::lifecycle::policy::REINITIALIZATION_DELAY_MS;

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
) -> Result<(), StorageError>
where
    R: StorageLifecycleControl,
{
    let mut attempt = 0;
    loop {
        let result = if attempt == 0 {
            reader.initialize()
        } else {
            reader.reinitialize()
        };
        match result {
            Ok(()) => return Ok(()),
            Err(StorageError::CardRemoved) if attempt + 1 < REINITIALIZATION_ATTEMPTS => {
                attempt += 1;
                logging::info(
                    logging::BOOT_SUBSYSTEM,
                    format_args!(
                        "[STORAGE] Card unavailable; bounded reinitialization attempt {}/{}",
                        attempt + 1,
                        REINITIALIZATION_ATTEMPTS
                    ),
                );
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
            initialize_with_recovery(reader, board)?;
            reader.read_block(crate::drivers::BlockAddress::new(0), block)
        }
        Err(error) => Err(error),
    }
}
