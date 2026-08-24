//! Kernel startup orchestration.

mod lifecycle;
mod loading;
mod startup;
mod storage;

use crate::{logging, platform};

/// Storage state and the reader retained for runtime card recovery.
pub(super) struct StorageRuntime {
    /// Stores the `status` value for this bounded state.
    status: lifecycle::status::StorageStatus,
    #[cfg(feature = "sdio")]
    /// Stores the `recovery_reader` value for this bounded state.
    recovery_reader: Option<platform::PlatformSdioReader>,
    #[cfg(feature = "sdio")]
    /// Stores the `recovery_retry_log_count` value for this bounded state.
    recovery_retry_log_count: u32,
    #[cfg(feature = "sdio")]
    /// Stores the `recovery_probe_count` value for this bounded state.
    recovery_probe_count: u32,
}

impl StorageRuntime {
    /// Builds storage recovery state with an attached SDIO reader.
    pub(super) const fn from_status(status: lifecycle::status::StorageStatus) -> Self {
        Self::new(status)
    }

    /// Creates storage recovery state without an attached reader.
    pub(super) const fn new(status: lifecycle::status::StorageStatus) -> Self {
        Self {
            status,
            #[cfg(feature = "sdio")]
            recovery_reader: None,
            #[cfg(feature = "sdio")]
            recovery_retry_log_count: 0,
            #[cfg(feature = "sdio")]
            recovery_probe_count: 0,
        }
    }

    #[cfg(feature = "sdio")]
    /// Creates storage recovery state with an attached SDIO reader.
    pub(super) const fn with_recovery_reader(
        status: lifecycle::status::StorageStatus,
        reader: platform::PlatformSdioReader,
    ) -> Self {
        Self {
            status,
            recovery_reader: Some(reader),
            recovery_retry_log_count: 0,
            recovery_probe_count: 0,
        }
    }

    /// Returns the current storage lifecycle status.
    pub(super) const fn status(&self) -> lifecycle::status::StorageStatus {
        self.status
    }

    #[cfg(feature = "sdio")]
    /// Performs the `poll_recovery` operation for this subsystem.
    pub(super) fn poll_recovery(&mut self, board: &mut platform::Platform) {
        storage::poll_runtime(self, board);
    }

    #[cfg(feature = "sdio")]
    /// Reports whether the bounded recovery poll interval has elapsed.
    pub(super) const fn recovery_poll_due(&self, elapsed_ms: u32) -> bool {
        elapsed_ms >= crate::drivers::lifecycle::policy::RECOVERY_POLL_PERIOD_MS
    }
}

/// Runs the kernel bootstrap sequence and enters the heartbeat loop.
pub fn run() -> ! {
    let mut board = platform::initialize();
    #[cfg(feature = "abi-mpu")]
    if let Some(layout) = platform::ISOLATION_LAYOUT {
        platform::mpu::configure_hardware(layout);
    }

    startup::initialize_logging();
    let reset_cause = board.reset_cause();
    logging::info(
        logging::BOOT_SUBSYSTEM,
        format_args!("[BOOT] Reset cause: {:?}", reset_cause),
    );
    #[cfg(feature = "abi-current")]
    crate::security::fault::report_persistent();
    let boot_mode = lifecycle::status::BootMode::from_reset_cause(reset_cause);
    if boot_mode == lifecycle::status::BootMode::SafeMode {
        logging::error(
            logging::SECURITY_SUBSYSTEM,
            format_args!(
                "[RECOVERY] Watchdog reset detected; entering Safe Mode and skipping application launch"
            ),
        );
    }
    #[cfg(feature = "usb-cdc")]
    if let Some(resources) = board.take_usb_resources() {
        logging::initialize_usb(
            resources,
            boot_mode == lifecycle::status::BootMode::SafeMode,
            &mut board,
        );
    } else {
        logging::error(
            logging::BOOT_SUBSYSTEM,
            format_args!("[USB] USB resources unavailable"),
        );
    }

    #[cfg(feature = "driver-hardware-test")]
    unsafe {
        // SAFETY: All test-build interrupt handlers are linked, and USB/logging
        // ownership is initialized before EXTI events can emit diagnostics.
        cortex_m::interrupt::enable();
    }

    startup::emit_boot_banner();
    logging::info(
        logging::BOOT_SUBSYSTEM,
        format_args!("[BOOT] System clock: {} MHz", platform::SYSTEM_CLOCK_MHZ),
    );
    logging::info(
        logging::BOOT_SUBSYSTEM,
        format_args!("Hardware bootstrap complete"),
    );

    #[cfg(feature = "driver-hardware-test")]
    platform::run_driver_timeout_probe(&mut board);

    #[cfg(feature = "abi-context-switch")]
    if let Err(error) = crate::security::scheduling::initialize() {
        logging::error(
            logging::SECURITY_SUBSYSTEM,
            format_args!("[SECURITY] Scheduler initialization failed: {:?}", error),
        );
    }

    // Keep a visible indication active while storage initialization is in progress.
    board.set_status_led(true);
    logging::info(
        logging::BOOT_SUBSYSTEM,
        format_args!("[STORAGE] Starting storage initialization"),
    );
    let storage_runtime = storage::initialize(&mut board, boot_mode);

    logging::info(
        logging::BOOT_SUBSYSTEM,
        format_args!("Entering kernel heartbeat"),
    );

    lifecycle::heartbeat::run(board, storage_runtime);
}
