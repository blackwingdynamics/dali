//! Kernel startup orchestration.

mod heartbeat;

use crate::{board, logging};
#[cfg(feature = "board-stm32f405-sd")]
use crate::{
    drivers::sdio::SdioBlockReader,
    storage::{BLOCK_SIZE, Block, BlockAddress, BlockReader},
};
use stm32f4xx_hal::pac;

/// Runs the kernel bootstrap sequence and enters the heartbeat loop.
pub fn run() -> ! {
    initialize_logging();
    emit_boot_banner();

    // The reset entry point runs once, so both peripheral singleton tokens are available.
    let device = pac::Peripherals::take().unwrap();
    let core = cortex_m::Peripherals::take().unwrap();
    let mut board = board::initialize(device, core);

    initialize_storage(&mut board);

    logging::info(
        logging::BOOT_SUBSYSTEM,
        format_args!("[BOOT] System clock: {} MHz", board::SYSTEM_CLOCK_MHZ),
    );
    logging::info(
        logging::BOOT_SUBSYSTEM,
        format_args!("Hardware bootstrap complete"),
    );
    logging::info(
        logging::BOOT_SUBSYSTEM,
        format_args!("Entering kernel heartbeat"),
    );

    heartbeat::run(board);
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

#[cfg(feature = "board-stm32f405-sd")]
fn initialize_storage(board: &mut board::Board) {
    let Some((peripheral, pins, clocks)) = board.take_sdio_resources() else {
        logging::error(
            logging::BOOT_SUBSYSTEM,
            format_args!("[STORAGE] SDIO resources unavailable"),
        );
        return;
    };

    let mut reader = SdioBlockReader::new(peripheral, pins, clocks);
    if let Err(error) = reader.initialize() {
        logging::error(
            logging::BOOT_SUBSYSTEM,
            format_args!("[STORAGE] SDIO initialization failed: {:?}", error),
        );
        return;
    }

    logging::info(
        logging::BOOT_SUBSYSTEM,
        format_args!("[STORAGE] SDIO card initialized"),
    );

    let mut block: Block = [0; BLOCK_SIZE];
    match reader.read_block(BlockAddress::new(0), &mut block) {
        Ok(()) => logging::info(
            logging::BOOT_SUBSYSTEM,
            format_args!("[STORAGE] Read block 0 successfully"),
        ),
        Err(error) => logging::error(
            logging::BOOT_SUBSYSTEM,
            format_args!("[STORAGE] Block 0 read failed: {:?}", error),
        ),
    }
}

#[cfg(not(feature = "board-stm32f405-sd"))]
fn initialize_storage(_board: &mut board::Board) {}
