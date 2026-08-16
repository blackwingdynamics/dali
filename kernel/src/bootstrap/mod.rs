//! Kernel startup orchestration.

mod heartbeat;
mod status;

use crate::{board, logging};
#[cfg(feature = "sdio")]
use crate::{
    drivers::sdio::SdioBlockReader,
    storage::{BLOCK_SIZE, Block, BlockAddress, BlockReader},
};
use stm32f4xx_hal::pac;

/// Runs the kernel bootstrap sequence and enters the heartbeat loop.
pub fn run() -> ! {
    // The reset entry point runs once, so both peripheral singleton tokens are available.
    let device = pac::Peripherals::take().unwrap();
    let core = cortex_m::Peripherals::take().unwrap();
    let mut board = board::initialize(device, core);

    initialize_logging();
    #[cfg(feature = "usb-cdc")]
    if let Some(resources) = board.take_usb_resources() {
        logging::initialize_usb(resources);
    } else {
        logging::error(
            logging::BOOT_SUBSYSTEM,
            format_args!("[USB] USB resources unavailable"),
        );
    }

    emit_boot_banner();
    logging::info(
        logging::BOOT_SUBSYSTEM,
        format_args!("[BOOT] System clock: {} MHz", board::SYSTEM_CLOCK_MHZ),
    );
    logging::info(
        logging::BOOT_SUBSYSTEM,
        format_args!("Hardware bootstrap complete"),
    );

    // Keep a visible indication active while storage initialization is in progress.
    board::set_status_led(&mut board, true);
    logging::info(
        logging::BOOT_SUBSYSTEM,
        format_args!("[STORAGE] Starting storage initialization"),
    );
    let storage_status = initialize_storage(&mut board);

    logging::info(
        logging::BOOT_SUBSYSTEM,
        format_args!("Entering kernel heartbeat"),
    );

    heartbeat::run(board, storage_status);
}

fn initialize_logging() {
    logging::initialize();
}

fn emit_boot_banner() {
    logging::info(
        logging::BOOT_SUBSYSTEM,
        format_args!("===================================="),
    );
    logging::info(
        logging::BOOT_SUBSYSTEM,
        format_args!("   Dali OS Kernel Booting...       "),
    );
    logging::info(
        logging::BOOT_SUBSYSTEM,
        format_args!("===================================="),
    );
}

#[cfg(feature = "sdio")]
fn initialize_storage(board: &mut board::Board) -> status::StorageStatus {
    let Some((peripheral, pins, clocks)) = board.take_sdio_resources() else {
        logging::error(
            logging::BOOT_SUBSYSTEM,
            format_args!("[STORAGE] SDIO resources unavailable"),
        );
        return status::StorageStatus::Failure;
    };

    let mut reader = SdioBlockReader::new(peripheral, pins, clocks);
    if let Err(error) = reader.initialize() {
        logging::error(
            logging::BOOT_SUBSYSTEM,
            format_args!("[STORAGE] SDIO initialization failed: {:?}", error),
        );
        return match error {
            crate::storage::StorageError::NotReady | crate::storage::StorageError::Timeout => {
                status::StorageStatus::NotDetected
            }
            _ => status::StorageStatus::Failure,
        };
    }

    logging::info(
        logging::BOOT_SUBSYSTEM,
        format_args!("[STORAGE] SDIO card initialized"),
    );

    let mut block: Block = [0; BLOCK_SIZE];
    match reader.read_block(BlockAddress::new(0), &mut block) {
        Ok(()) => {
            logging::info(
                logging::BOOT_SUBSYSTEM,
                format_args!("[STORAGE] Read block 0 successfully"),
            );
            let package = if board::APPLICATION_EXECUTION_SUPPORTED {
                crate::loader::load_amrn_file(reader)
            } else {
                crate::loader::validate_amrn_file(reader)
            };
            match package {
                Ok(package) => {
                    logging::info(
                        logging::BOOT_SUBSYSTEM,
                        format_args!("[LOADER] AMRN header and payload validated"),
                    );
                    if board::APPLICATION_EXECUTION_SUPPORTED {
                        crate::loader::start_application(package);
                    }
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
fn initialize_storage(_board: &mut board::Board) -> status::StorageStatus {
    status::StorageStatus::NotDetected
}
