//! Kernel-owned scheduler initialization before interrupt integration.

use crate::runtime::scheduling::{
    scheduler::{Scheduler, SchedulerError},
    storage::{SchedulerStorage, SchedulerStorageError},
};

type TargetScheduler = Scheduler<{ crate::platform::CONTEXT_CAPACITY }>;

static SCHEDULER_STORAGE: SchedulerStorage<TargetScheduler> = SchedulerStorage::new();

#[cfg(feature = "abi-context-switch")]
use cortex_m_rt::exception;

/// Errors returned while initializing the target scheduler state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SchedulerInitializationError {
    /// The target profile does not declare scheduler configuration.
    MissingProfile,
    /// The declared quantum could not construct a scheduler.
    Scheduler(SchedulerError),
    /// The kernel-owned storage rejected initialization.
    Storage(SchedulerStorageError),
}

/// Initializes scheduler state once from the selected target profile.
pub(crate) fn initialize() -> Result<(), SchedulerInitializationError> {
    let profile =
        crate::platform::SCHEDULER_PROFILE.ok_or(SchedulerInitializationError::MissingProfile)?;
    let scheduler = TargetScheduler::new(profile.quantum_ticks)
        .map_err(SchedulerInitializationError::Scheduler)?;
    SCHEDULER_STORAGE
        .initialize(scheduler)
        .map_err(SchedulerInitializationError::Storage)
}

/// Accounts for one target-provided SysTick interrupt.
///
/// PendSV is requested only when the scheduler has both an active context and
/// another ready context. This prevents the feature-gated vector from
/// attempting an exception return before application contexts are wired.
#[cfg(feature = "abi-context-switch")]
pub(crate) fn on_systick() {
    let scheduler = unsafe {
        // SAFETY: SysTick is a single exception context. PendSV is the only
        // other scheduler access path and remains deferred until this handler
        // returns, so the mutable operation is exclusive.
        match SCHEDULER_STORAGE.get_mut_ptr() {
            Ok(pointer) => &mut *pointer,
            Err(_) => return,
        }
    };
    scheduler.on_tick();
    if scheduler.switch_requested() {
        cortex_m::peripheral::SCB::set_pendsv();
    }
}

#[cfg(feature = "abi-context-switch")]
#[exception]
fn SysTick() {
    on_systick();
}
