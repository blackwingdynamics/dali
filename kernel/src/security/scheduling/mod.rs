//! Kernel-owned scheduler initialization before interrupt integration.

use crate::runtime::scheduling::{
    record::ScheduledContext,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SchedulerAccessError {
    /// The scheduler storage was not ready for an operation.
    Storage(SchedulerStorageError),
    /// The scheduler rejected the requested context transition.
    Scheduler(SchedulerError),
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

/// Applies one exclusive operation to the initialized scheduler state.
fn with_scheduler<R>(
    operation: impl FnOnce(&mut TargetScheduler) -> R,
) -> Result<R, SchedulerStorageError> {
    let pointer = unsafe {
        // SAFETY: Callers run during bootstrap or a scheduler exception path
        // with scheduler access serialized by the interrupt boundary.
        SCHEDULER_STORAGE.get_mut_ptr()?
    };
    Ok(operation(unsafe {
        // SAFETY: `get_mut_ptr` verifies initialization and the caller owns
        // the exclusive scheduler access for the complete operation.
        &mut *pointer
    }))
}

/// Registers validated application launch contexts in manifest order.
pub(crate) fn register_contexts(
    contexts: impl IntoIterator<Item = ScheduledContext>,
) -> Result<(), SchedulerAccessError> {
    with_scheduler(|scheduler| {
        contexts
            .into_iter()
            .try_for_each(|context| scheduler.insert(context).map(|_| ()))
    })
    .map_err(SchedulerAccessError::Storage)?
    .map_err(SchedulerAccessError::Scheduler)
}

/// Activates the first registered context without performing an exception return.
pub(crate) fn activate_first() -> Result<(), SchedulerAccessError> {
    with_scheduler(|scheduler| scheduler.activate_first())
        .map_err(SchedulerAccessError::Storage)?
        .map(|_| ())
        .map_err(SchedulerAccessError::Scheduler)
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
