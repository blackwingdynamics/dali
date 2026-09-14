//! Hardware-neutral platform composition boundary.
//!
//! A firmware composition crate provides the selected board services. This
//! module owns only composition and the typed facade consumed by kernel policy.

mod architecture;
#[cfg(feature = "abi-mpu")]
mod protection;
mod registry;
#[cfg(feature = "usb-cdc")]
mod usb;

#[cfg(any(feature = "abi-context-switch", feature = "abi-mpu"))]
pub(crate) use architecture::request_context_switch;
#[cfg(feature = "abi-test-fixtures")]
pub(crate) use architecture::write_fault_register;
#[cfg(feature = "abi-current")]
pub(crate) use architecture::{
    main_stack_pointer, process_stack_pointer, set_process_stack_pointer,
    wait_for_registered_interrupt,
};
pub(crate) use architecture::{read_fault_register, recover_to_kernel};
#[cfg(feature = "abi-mpu")]
pub(crate) use protection::{
    activate_application_regions, configure as configure_memory_protection,
};
#[cfg(any(feature = "abi-current", feature = "abi-mpu"))]
pub(crate) use registry::memory_profile;
#[cfg(feature = "abi-context-switch")]
pub(crate) use registry::scheduler_profile;
#[cfg(any(
    feature = "abi-authentication",
    feature = "repository-loader",
    feature = "abi-context-switch"
))]
pub(crate) use registry::target_profile;
#[cfg(feature = "abi-authentication")]
pub(crate) use registry::trust_anchors;
#[cfg(feature = "usb-cdc")]
pub(crate) use usb::{pend_irq as pend_usb_irq, service_irq as service_usb_irq};

use dali_kernel_api::{ArchitectureBackend, BoardBackend, BoardError, ResetCause, WatchdogBackend};

#[cfg(feature = "abi-context-switch")]
/// Kernel-owned watchdog callback used while an application context is active.
static SCHEDULER_WATCHDOG: critical_section::Mutex<core::cell::RefCell<Option<WatchdogService>>> =
    critical_section::Mutex::new(core::cell::RefCell::new(None));

#[cfg(feature = "abi-context-switch")]
/// Kernel-owned watchdog callback registered for application context switches.
#[derive(Clone, Copy)]
struct WatchdogService {
    /// Opaque address of the platform facade that owns the watchdog.
    platform: usize,
    /// Backend-independent callback used to feed the watchdog.
    feed: unsafe fn(usize) -> bool,
}

/// Failure returned by the kernel-owned watchdog service boundary.
#[derive(Debug)]
pub(crate) enum WatchdogServiceError {
    /// The watchdog runtime was already installed.
    #[cfg(feature = "sdio")]
    AlreadyInstalled,
    /// Feeding the watchdog failed.
    Feed,
}

/// Kernel facade over one externally selected board backend.
pub(crate) struct Platform<B>
where
    B: BoardBackend,
    B::Watchdog: WatchdogBackend,
{
    /// Selected board implementation, kept behind the kernel facade.
    backend: B,
    /// Optional watchdog runtime owned by the kernel heartbeat.
    watchdog: Option<crate::runtime::watchdog::WatchdogRuntime<B::Watchdog>>,
}

impl<B> Platform<B>
where
    B: BoardBackend,
    B::Watchdog: WatchdogBackend,
{
    /// Initializes the externally selected backend and its registered ports.
    pub(crate) fn initialize() -> Result<Self, BoardError> {
        registry::register::<B>()?;
        architecture::register::<B>();
        #[cfg(feature = "abi-mpu")]
        protection::register::<B>();
        #[cfg(feature = "usb-cdc")]
        usb::register::<B>();
        let _ = read_fault_register as fn(dali_kernel_api::FaultRegister) -> u32;
        let _ = recover_to_kernel as unsafe fn(u32) -> !;
        Ok(Self {
            backend: B::initialize()?,
            watchdog: None,
        })
    }

    /// Returns the selected backend metadata.
    pub(crate) fn info() -> dali_kernel_api::BoardInfo {
        B::info()
    }

    /// Returns the manifest-owned application execution policy.
    #[cfg(all(feature = "sdio", not(feature = "abi-current")))]
    pub(crate) fn supports_application_execution() -> bool {
        B::info().target.application_supported
    }

    /// Returns the reset cause captured by the backend.
    pub(crate) fn reset_cause(&self) -> ResetCause {
        self.backend.reset_cause()
    }

    /// Waits through the selected architecture backend.
    pub(crate) fn wait_for_interrupt()
    where
        B::Architecture: ArchitectureBackend,
    {
        B::Architecture::wait_for_interrupt()
    }

    /// Enables interrupts through the selected architecture backend.
    #[cfg(any(
        feature = "driver-hardware-test",
        feature = "abi-context-switch",
        feature = "usb-cdc"
    ))]
    pub(crate) fn enable_interrupts()
    where
        B::Architecture: ArchitectureBackend,
    {
        B::Architecture::enable_interrupts();
    }
}

impl<B> Platform<B>
where
    B: BoardBackend,
    B::Watchdog: WatchdogBackend,
{
    /// Sets the backend-owned status indicator.
    pub(crate) fn set_status_led(&mut self, on: bool) -> Result<(), BoardError> {
        self.backend.set_status_led(on)
    }

    /// Delays through the backend-owned timer.
    pub(crate) fn delay_ms(&mut self, milliseconds: u32) -> Result<(), BoardError> {
        self.backend.delay_ms(milliseconds)
    }

    /// Polls backend-owned user input.
    pub(crate) fn poll_user_key(&mut self) {
        self.backend.poll_user_key();
    }

    /// Initializes board-owned USB state without exposing its concrete types.
    #[cfg(feature = "usb-cdc")]
    pub(crate) fn initialize_usb(&mut self, force_reenumeration: bool) -> bool {
        self.backend.initialize_usb(force_reenumeration)
    }

    /// Takes the initialized storage reader.
    #[cfg(feature = "sdio")]
    pub(crate) fn take_sdio_reader(&mut self) -> Option<B::StorageReader> {
        self.backend.take_storage_reader().ok()
    }

    /// Transfers the backend-owned watchdog into the kernel runtime.
    #[cfg(feature = "sdio")]
    pub(crate) fn take_watchdog(&mut self) -> Result<B::Watchdog, BoardError> {
        self.backend.take_watchdog()
    }

    /// Installs a watchdog runtime after its ownership contract is armed.
    #[cfg(feature = "sdio")]
    pub(crate) fn install_watchdog(
        &mut self,
        runtime: crate::runtime::watchdog::WatchdogRuntime<B::Watchdog>,
    ) -> Result<(), WatchdogServiceError> {
        if self.watchdog.is_some() {
            return Err(WatchdogServiceError::AlreadyInstalled);
        }
        self.watchdog = Some(runtime);
        #[cfg(feature = "abi-context-switch")]
        critical_section::with(|cs| {
            let mut service = SCHEDULER_WATCHDOG.borrow(cs).borrow_mut();
            if service.is_none() {
                *service = Some(WatchdogService {
                    platform: self as *mut Self as usize,
                    feed: feed_watchdog::<B>,
                });
            }
        });
        Ok(())
    }

    /// Feeds the installed watchdog from the kernel heartbeat.
    pub(crate) fn service_watchdog(&mut self) -> Result<(), WatchdogServiceError> {
        let Some(runtime) = self.watchdog.as_mut() else {
            return Ok(());
        };
        runtime
            .feed(crate::runtime::watchdog::FeedOwner::KernelHeartbeat)
            .map_err(|_| WatchdogServiceError::Feed)
    }

    /// Enables the selected backend scheduler tick.
    #[cfg(feature = "abi-context-switch")]
    pub(crate) fn enable_scheduler_tick(&mut self, tick_hz: u32) -> bool {
        self.backend.enable_scheduler_tick(tick_hz)
    }

    /// Runs the selected backend hardware acceptance probe.
    #[cfg(feature = "driver-hardware-test")]
    pub(crate) fn run_driver_timeout_probe(&mut self) {
        self.backend.run_driver_timeout_probe();
    }
}

#[cfg(feature = "abi-context-switch")]
/// Feeds the installed watchdog from the kernel-owned scheduler exception.
pub(crate) fn service_watchdog_from_scheduler() {
    critical_section::with(|cs| {
        let service = SCHEDULER_WATCHDOG.borrow(cs).borrow();
        if let Some(service) = *service {
            let _ = unsafe { (service.feed)(service.platform) };
        }
    });
}

#[cfg(feature = "abi-context-switch")]
/// Feeds the watchdog through the platform facade stored in `platform`.
///
/// # Safety
///
/// `platform` must be the address of a live `Platform<B>` value whose watchdog
/// runtime is installed.
unsafe fn feed_watchdog<B>(platform: usize) -> bool
where
    B: BoardBackend,
    B::Watchdog: WatchdogBackend,
{
    let platform = unsafe { &mut *(platform as *mut Platform<B>) };
    platform.service_watchdog().is_ok()
}
