//! Hardware-neutral platform composition boundary.
//!
//! A firmware composition crate must provide the selected board services. The
//! kernel core does not select, import, or name any board backend.

#[cfg(any(
    feature = "abi-current",
    feature = "abi-mpu",
    feature = "abi-context-switch"
))]
use dali_kernel_api::BoardInfo;
use dali_kernel_api::{ArchitectureBackend, BoardBackend, BoardError, ResetCause, WatchdogBackend};

/// Architecture operations registered by the selected firmware composition.
static ARCHITECTURE: critical_section::Mutex<
    core::cell::RefCell<Option<dali_kernel_api::ArchitectureOperations>>,
> = critical_section::Mutex::new(core::cell::RefCell::new(None));

/// USB operations registered by the selected firmware composition.
#[cfg(feature = "usb-cdc")]
static USB_OPERATIONS: critical_section::Mutex<
    core::cell::RefCell<Option<dali_kernel_api::UsbOperations>>,
> = critical_section::Mutex::new(core::cell::RefCell::new(None));

/// Memory-protection operations registered by the selected backend.
#[cfg(feature = "abi-mpu")]
static MEMORY_PROTECTION: critical_section::Mutex<
    core::cell::RefCell<Option<dali_kernel_api::MemoryProtectionOperations>>,
> = critical_section::Mutex::new(core::cell::RefCell::new(None));

#[cfg(any(
    feature = "abi-current",
    feature = "abi-mpu",
    feature = "abi-context-switch"
))]
/// Backend metadata registered during kernel bootstrap.
static BOARD_INFO: critical_section::Mutex<core::cell::RefCell<Option<BoardInfo>>> =
    critical_section::Mutex::new(core::cell::RefCell::new(None));

#[cfg(feature = "abi-context-switch")]
/// Kernel-owned watchdog callback used while an application context is active.
static SCHEDULER_WATCHDOG: critical_section::Mutex<core::cell::RefCell<Option<WatchdogService>>> =
    critical_section::Mutex::new(core::cell::RefCell::new(None));

#[cfg(feature = "abi-context-switch")]
#[derive(Clone, Copy)]
struct WatchdogService {
    platform: usize,
    feed: unsafe fn(usize) -> bool,
}

/// Returns metadata registered by the selected firmware composition.
#[cfg(any(
    feature = "abi-current",
    feature = "abi-mpu",
    feature = "abi-context-switch"
))]
pub(crate) fn board_info() -> Option<BoardInfo> {
    critical_section::with(|cs| *BOARD_INFO.borrow(cs).borrow())
}

/// Returns the target profile registered by the selected firmware composition.
#[cfg(any(
    feature = "abi-authentication",
    feature = "repository-loader",
    feature = "abi-context-switch"
))]
pub(crate) fn target_profile() -> Option<&'static dali_targets::TargetProfile> {
    board_info().map(|info| info.target)
}

/// Returns the memory profile registered by the selected firmware composition.
#[cfg(any(feature = "abi-current", feature = "abi-mpu"))]
pub(crate) fn memory_profile() -> Option<dali_targets::MemoryProfile> {
    board_info().map(|info| info.memory)
}

/// Returns scheduler metadata registered by the selected firmware composition.
#[cfg(feature = "abi-context-switch")]
pub(crate) fn scheduler_profile() -> Option<dali_targets::SchedulerProfile> {
    target_profile().and_then(|target| target.scheduler)
}

/// Returns trust anchors selected by the registered target and build policy.
#[cfg(feature = "abi-authentication")]
pub(crate) fn trust_anchors() -> &'static [dali_targets::TrustAnchorProfile] {
    let Some(target) = target_profile() else {
        return &[];
    };
    #[cfg(feature = "abi-test-fixtures")]
    {
        target.authentication.development_trust_anchors
    }
    #[cfg(not(feature = "abi-test-fixtures"))]
    {
        target.authentication.release_trust_anchors
    }
}

/// Failure returned by the kernel-owned watchdog service boundary.
#[derive(Debug)]
pub(crate) enum WatchdogServiceError {
    /// The watchdog runtime was already installed.
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
    /// Initializes the externally selected backend.
    pub(crate) fn initialize() -> Result<Self, BoardError> {
        let _ = read_fault_register as fn(dali_kernel_api::FaultRegister) -> u32;
        let _ = recover_to_kernel as unsafe fn(u32) -> !;
        critical_section::with(|cs| {
            let mut architecture = ARCHITECTURE.borrow(cs).borrow_mut();
            if architecture.is_none() {
                *architecture = Some(B::Architecture::operations());
            }
        });
        #[cfg(feature = "abi-mpu")]
        critical_section::with(|cs| {
            let mut protection = MEMORY_PROTECTION.borrow(cs).borrow_mut();
            if protection.is_none() {
                *protection = B::memory_protection_operations();
            }
        });
        #[cfg(any(
            feature = "abi-current",
            feature = "abi-mpu",
            feature = "abi-context-switch"
        ))]
        critical_section::with(|cs| {
            let mut info = BOARD_INFO.borrow(cs).borrow_mut();
            if info.is_none() {
                *info = Some(B::info());
            }
        });
        #[cfg(feature = "usb-cdc")]
        critical_section::with(|cs| {
            let mut operations = USB_OPERATIONS.borrow(cs).borrow_mut();
            if operations.is_none() {
                *operations = B::usb_operations();
            }
        });
        Ok(Self {
            backend: B::initialize()?,
            watchdog: None,
        })
    }

    /// Returns the selected backend metadata.
    pub(crate) fn info() -> dali_kernel_api::BoardInfo {
        B::info()
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
    #[cfg(feature = "driver-hardware-test")]
    pub(crate) fn enable_interrupts()
    where
        B::Architecture: ArchitectureBackend,
    {
        B::Architecture::enable_interrupts();
    }
}

/// Requests a deferred context switch through the installed architecture.
#[cfg(any(feature = "abi-context-switch", feature = "abi-mpu"))]
pub(crate) fn request_context_switch() {
    if let Some(operations) = critical_section::with(|cs| *ARCHITECTURE.borrow(cs).borrow()) {
        (operations.request_context_switch)();
    }
}

/// Waits through the installed architecture after bootstrap registration.
#[cfg(feature = "abi-current")]
pub(crate) fn wait_for_registered_interrupt() -> ! {
    if let Some(operations) = critical_section::with(|cs| *ARCHITECTURE.borrow(cs).borrow()) {
        (operations.wait_for_interrupt)();
    }
    loop {
        core::hint::spin_loop();
    }
}

/// Reads the registered architecture process stack pointer.
#[cfg(feature = "abi-current")]
pub(crate) fn process_stack_pointer() -> Option<u32> {
    critical_section::with(|cs| {
        ARCHITECTURE
            .borrow(cs)
            .borrow()
            .map(|operations| (operations.read_process_stack_pointer)())
    })
}

/// Writes the registered architecture process stack pointer.
#[cfg(feature = "abi-current")]
pub(crate) unsafe fn set_process_stack_pointer(value: u32) -> bool {
    let Some(operations) = critical_section::with(|cs| *ARCHITECTURE.borrow(cs).borrow()) else {
        return false;
    };
    // SAFETY: The caller validates the target stack invariant.
    unsafe { (operations.write_process_stack_pointer)(value) };
    true
}

/// Reads the registered architecture main stack pointer.
#[cfg(feature = "abi-current")]
pub(crate) fn main_stack_pointer() -> Option<u32> {
    critical_section::with(|cs| {
        ARCHITECTURE
            .borrow(cs)
            .borrow()
            .map(|operations| (operations.read_main_stack_pointer)())
    })
}

/// Reads a fault register through the selected architecture backend.
#[inline]
pub fn read_fault_register(register: dali_kernel_api::FaultRegister) -> u32 {
    critical_section::with(|cs| {
        ARCHITECTURE
            .borrow(cs)
            .borrow()
            .map_or(0, |operations| (operations.read_fault_register)(register))
    })
}

/// Writes a fault register through the selected architecture backend.
#[cfg(feature = "abi-test-fixtures")]
pub(crate) fn write_fault_register(register: dali_kernel_api::FaultRegister, value: u32) {
    if let Some(operations) = critical_section::with(|cs| *ARCHITECTURE.borrow(cs).borrow()) {
        (operations.write_fault_register)(register, value);
    }
}

/// Returns from a prepared fault-recovery frame through the backend.
///
/// # Safety
///
/// `frame_address` must point to a validated kernel-owned recovery frame.
#[inline(never)]
pub unsafe fn recover_to_kernel(frame_address: u32) -> ! {
    if let Some(operations) = critical_section::with(|cs| *ARCHITECTURE.borrow(cs).borrow()) {
        // SAFETY: The caller has built a validated kernel-owned recovery frame.
        unsafe { (operations.recover_to_kernel)(frame_address) };
    }
    loop {
        core::hint::spin_loop();
    }
}

/// Configures the initial protection map through the selected backend.
#[cfg(feature = "abi-mpu")]
pub(crate) fn configure_memory_protection(memory: dali_targets::MemoryProfile) -> bool {
    let Some(operations) = critical_section::with(|cs| *MEMORY_PROTECTION.borrow(cs).borrow())
    else {
        return false;
    };
    (operations.configure)(memory);
    true
}

/// Activates application permissions through the selected backend.
#[cfg(feature = "abi-mpu")]
pub(crate) fn activate_application_regions(slot: dali_targets::IsolationSlot) -> bool {
    let Some(operations) = critical_section::with(|cs| *MEMORY_PROTECTION.borrow(cs).borrow())
    else {
        return false;
    };
    (operations.activate_application_regions)(slot)
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
    pub(crate) fn take_watchdog(&mut self) -> Result<B::Watchdog, BoardError> {
        self.backend.take_watchdog()
    }

    /// Installs a watchdog runtime after its ownership contract is armed.
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
unsafe fn feed_watchdog<B>(platform: usize) -> bool
where
    B: BoardBackend,
    B::Watchdog: WatchdogBackend,
{
    let platform = unsafe { &mut *(platform as *mut Platform<B>) };
    platform.service_watchdog().is_ok()
}

/// Services the registered board USB backend through a kernel-owned callback.
#[cfg(feature = "usb-cdc")]
pub(crate) fn service_usb_irq(drain: fn(dali_usb::LinkState, &mut dyn dali_usb::ByteSink)) {
    if let Some(operations) = critical_section::with(|cs| *USB_OPERATIONS.borrow(cs).borrow()) {
        (operations.service_irq)(drain);
    }
}

/// Pends the registered board USB interrupt.
#[cfg(feature = "usb-cdc")]
pub(crate) fn pend_usb_irq() {
    if let Some(operations) = critical_section::with(|cs| *USB_OPERATIONS.borrow(cs).borrow()) {
        (operations.pend_irq)();
    }
}
