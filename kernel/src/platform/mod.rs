//! Platform selection boundary for hardware-specific kernel entry points.

use core::cell::UnsafeCell;

#[cfg(feature = "board-stm32f405-sd")]
mod f405;

/// Stable operations supplied by a selected platform backend.
pub(crate) trait Backend: Sized {
    /// Human-readable system clock value exposed by the boot status path.
    const SYSTEM_CLOCK_MHZ: u32;

    #[cfg(feature = "sdio")]
    /// Generic block reader supplied by the platform SDIO transport.
    type SdioReader: crate::drivers::BlockReader + crate::drivers::StorageLifecycleControl;

    #[cfg(feature = "usb-cdc")]
    /// Board-owned resources used to construct the USB bus.
    type UsbResources: crate::logging::usb_cdc::UsbResources;

    /// Takes the platform singletons and initializes the backend.
    fn initialize() -> Self;

    /// Sets the board status indicator.
    fn set_status_led(&mut self, on: bool);

    /// Delays for a bounded number of milliseconds.
    fn delay_ms(&mut self, milliseconds: u32);

    /// Enables the target scheduler tick after application activation.
    #[cfg(feature = "abi-context-switch")]
    fn enable_scheduler_tick(&mut self, tick_hz: u32) -> bool;

    #[cfg(feature = "sdio")]
    /// Transfers the SDIO reader to the storage policy.
    fn take_sdio_reader(&mut self) -> Option<Self::SdioReader>;

    #[cfg(feature = "usb-cdc")]
    /// Transfers USB resources to the logging backend.
    fn take_usb_resources(&mut self) -> Option<Self::UsbResources>;

    /// Enables the backend's USB interrupt after initialization.
    #[cfg(feature = "usb-cdc")]
    fn unmask_usb_irq();

    /// Pends the backend's USB interrupt after a log enqueue.
    #[cfg(feature = "usb-cdc")]
    fn pend_usb_irq();
}

/// Stable kernel-facing facade over the selected platform backend.
pub(crate) struct Platform(f405::Board);

/// Watchdog runtime selected by the active platform backend.
pub(crate) type WatchdogRuntime = crate::runtime::watchdog::WatchdogRuntime<f405::F405Watchdog>;

/// Failure returned by the kernel-owned watchdog service boundary.
#[derive(Debug)]
pub(crate) enum WatchdogServiceError {
    /// The watchdog could not be armed before runtime ownership was installed.
    Arm,
    /// The watchdog runtime was already installed.
    AlreadyInstalled,
    /// Feeding the watchdog failed and feeding was disabled.
    Feed,
}

struct WatchdogStorage(UnsafeCell<Option<WatchdogRuntime>>);

// SAFETY: The storage is accessed only inside a critical section. The
// watchdog service is called from bootstrap, heartbeat, or SysTick, so no two
// callers can create mutable access concurrently.
unsafe impl Sync for WatchdogStorage {}

static WATCHDOG_STORAGE: WatchdogStorage = WatchdogStorage(UnsafeCell::new(None));

/// Watchdog profile supplied by the selected target manifest.
pub(crate) const WATCHDOG_PROFILE: Option<dali_targets::WatchdogProfile> =
    dali_targets::TARGET_F405.watchdog;

#[cfg(all(feature = "abi-authentication", feature = "abi-test-fixtures"))]
pub(crate) const TRUST_ANCHORS: &[dali_targets::TrustAnchorProfile] = dali_targets::TARGET_F405
    .authentication
    .development_trust_anchors;

#[cfg(all(feature = "abi-authentication", not(feature = "abi-test-fixtures")))]
pub(crate) const TRUST_ANCHORS: &[dali_targets::TrustAnchorProfile] = dali_targets::TARGET_F405
    .authentication
    .release_trust_anchors;

#[cfg(feature = "board-stm32f405-sd")]
pub(crate) const SYSTEM_CLOCK_MHZ: u32 = <f405::Board as Backend>::SYSTEM_CLOCK_MHZ;

#[cfg(feature = "sdio")]
pub(crate) type PlatformSdioReader = <f405::Board as Backend>::SdioReader;

#[cfg(feature = "board-stm32f405-sd")]
pub(crate) fn initialize() -> Platform {
    Platform(<f405::Board as Backend>::initialize())
}

/// Arms and installs the single kernel-owned watchdog before storage loading.
pub(crate) fn install_watchdog(mut runtime: WatchdogRuntime) -> Result<(), WatchdogServiceError> {
    runtime
        .arm(crate::runtime::watchdog::FeedOwner::KernelHeartbeat)
        .map_err(|_| WatchdogServiceError::Arm)?;
    cortex_m::interrupt::free(|_| unsafe {
        let storage = &mut *WATCHDOG_STORAGE.0.get();
        if storage.is_some() {
            return Err(WatchdogServiceError::AlreadyInstalled);
        }
        *storage = Some(runtime);
        Ok(())
    })
}

/// Feeds the installed watchdog from a kernel-owned execution boundary.
pub(crate) fn service_watchdog() -> Result<(), WatchdogServiceError> {
    cortex_m::interrupt::free(|_| unsafe {
        let storage = &mut *WATCHDOG_STORAGE.0.get();
        let Some(runtime) = storage.as_mut() else {
            return Ok(());
        };
        match runtime.feed(crate::runtime::watchdog::FeedOwner::KernelHeartbeat) {
            Ok(()) => Ok(()),
            Err(_) => {
                *storage = None;
                Err(WatchdogServiceError::Feed)
            }
        }
    })
}

/// Adapts the kernel watchdog boundary to repository chunk progress.
#[cfg(feature = "repository-loader")]
pub(crate) fn pet_repository_chunk() -> Result<(), crate::drivers::StorageError> {
    service_watchdog().map_err(|_| crate::drivers::StorageError::Transport)
}

/// Reports whether a repository verification progress feed succeeded.
#[cfg(feature = "repository-loader")]
pub(crate) fn repository_verification_progress() -> bool {
    pet_repository_chunk().is_ok()
}

#[cfg(feature = "board-stm32f405-sd")]
impl Platform {
    pub(crate) fn set_status_led(&mut self, on: bool) {
        self.0.set_status_led(on);
    }

    pub(crate) fn delay_ms(&mut self, milliseconds: u32) {
        self.0.delay_ms(milliseconds);
    }

    pub(crate) fn reset_cause(&self) -> crate::runtime::watchdog::ResetCause {
        self.0.reset_cause()
    }

    pub(crate) fn take_watchdog(&mut self) -> Option<f405::F405Watchdog> {
        self.0.take_watchdog()
    }

    #[cfg(feature = "abi-context-switch")]
    pub(crate) fn enable_scheduler_tick(&mut self, tick_hz: u32) -> bool {
        self.0.enable_scheduler_tick(tick_hz)
    }

    #[cfg(feature = "sdio")]
    pub(crate) fn take_sdio_reader(&mut self) -> Option<<f405::Board as Backend>::SdioReader> {
        self.0.take_sdio_reader()
    }

    #[cfg(feature = "usb-cdc")]
    pub(crate) fn take_usb_resources(&mut self) -> Option<UsbResources> {
        <f405::Board as Backend>::take_usb_resources(&mut self.0)
    }
}

#[cfg(feature = "usb-cdc")]
pub(crate) type UsbResources = <f405::Board as Backend>::UsbResources;

#[cfg(feature = "usb-cdc")]
pub(crate) fn unmask_usb_irq() {
    <f405::Board as Backend>::unmask_usb_irq();
}

#[cfg(feature = "usb-cdc")]
pub(crate) fn pend_usb_irq() {
    <f405::Board as Backend>::pend_usb_irq();
}

#[cfg(all(feature = "board-stm32f405-sd", feature = "abi-current"))]
pub(crate) use f405::MEMORY_PROFILE;

#[cfg(all(feature = "board-stm32f405-sd", feature = "abi-mpu"))]
pub(crate) use f405::{ISOLATION_LAYOUT, activate_application_regions};

#[cfg(all(feature = "board-stm32f405-sd", feature = "abi-mpu"))]
pub(crate) use crate::security::mpu;

#[cfg(all(feature = "board-stm32f405-sd", not(feature = "abi-current")))]
pub(crate) use f405::APPLICATION_EXECUTION_SUPPORTED;

#[cfg(all(feature = "board-stm32f405-sd", feature = "abi-current"))]
pub(crate) use f405::TARGET_PROFILE;

#[cfg(feature = "board-stm32f405-sd")]
pub(crate) use f405::DMA_REGION;

#[cfg(all(
    feature = "board-stm32f405-sd",
    feature = "abi-context-switch",
    feature = "abi-current"
))]
pub(crate) use f405::{CONTEXT_CAPACITY, SCHEDULER_PROFILE};

#[cfg(not(feature = "board-stm32f405-sd"))]
compile_error!("Select a supported Dali OS platform feature");
