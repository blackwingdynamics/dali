//! Kernel-owned scheduler initialization before interrupt integration.

use crate::runtime::scheduling::{
    scheduler::{Scheduler, SchedulerError},
    storage::{SchedulerStorage, SchedulerStorageError},
};

type TargetScheduler = Scheduler<{ crate::platform::CONTEXT_CAPACITY }>;

static SCHEDULER_STORAGE: SchedulerStorage<TargetScheduler> = SchedulerStorage::new();

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
