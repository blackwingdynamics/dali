//! Kernel-owned scheduler initialization before interrupt integration.

use crate::runtime::scheduling::{
    record::ScheduledContext,
    saved_state::{CALLEE_SAVED_REGISTER_COUNT, SavedContext},
    scheduler::{Scheduler, SchedulerError},
    storage::{SchedulerStorage, SchedulerStorageError},
};

/// Scheduler type sized from the active platform context profile.
const MAX_CONTEXT_CAPACITY: usize = 2;
/// Scheduler storage type selected by the active context capacity.
type TargetScheduler = Scheduler<MAX_CONTEXT_CAPACITY>;

/// Static scheduler storage used during the kernel lifecycle.
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Errors encountered while accessing the scheduler ownership boundary.
pub(crate) enum SchedulerAccessError {
    /// The scheduler storage was not ready for an operation.
    Storage(SchedulerStorageError),
    /// The scheduler rejected the requested context transition.
    Scheduler(SchedulerError),
}

/// Initializes scheduler state once from the selected target profile.
pub(crate) fn initialize() -> Result<(), SchedulerInitializationError> {
    let profile =
        crate::platform::scheduler_profile().ok_or(SchedulerInitializationError::MissingProfile)?;
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

/// Retires the faulted context and requests a protected PendSV restore.
#[cfg(all(feature = "abi-context-switch", target_arch = "arm"))]
pub(crate) fn recover_faulted_context() -> ! {
    let result = with_scheduler(|scheduler| -> Result<Option<()>, SchedulerError> {
        let Some(selection) = scheduler.terminate_active_and_select_next()? else {
            return Ok(None);
        };
        scheduler.arm_recovery_restore(selection.incoming)?;
        Ok(Some(()))
    });

    match result {
        Ok(Ok(Some(()))) => {
            crate::platform::request_context_switch();
            crate::platform::wait_for_registered_interrupt();
        }
        _ => crate::platform::wait_for_registered_interrupt(),
    }
}

/// Prepares the incoming CPU record for the privileged PendSV assembly path.
#[cfg(all(feature = "abi-context-switch", target_arch = "arm"))]
#[unsafe(export_name = "dali_kernel_prepare_pendsv")]
pub(crate) unsafe extern "C" fn prepare_pendsv(
    saved_registers: *const u32,
    psp: u32,
    control: u32,
    exception_return: u32,
) -> *const SavedContext {
    let saved = unsafe {
        // SAFETY: The naked PendSV wrapper passes a pointer to its aligned,
        // complete `r4..r11` scratch area on the kernel MSP.
        *(saved_registers as *const [u32; CALLEE_SAVED_REGISTER_COUNT])
    };
    let result = with_scheduler(|scheduler| {
        if let Some(incoming_id) = scheduler.take_recovery_target() {
            let incoming = scheduler.context(incoming_id)?;
            if !crate::platform::activate_application_regions(incoming.slot()) {
                return Err(SchedulerError::ProtectionUnavailable);
            }
            return scheduler.context_cpu_ptr(incoming_id);
        }
        let active = scheduler.active_context()?;
        let saved_context = ScheduledContext::new(
            SavedContext {
                psp,
                callee_saved: saved,
                control,
                exception_return,
            },
            active.slot(),
        );
        match scheduler.prepare_pendsv(saved_context)? {
            Some(selection) => {
                let incoming = scheduler.context(selection.incoming)?;
                if !crate::platform::activate_application_regions(incoming.slot()) {
                    return Err(SchedulerError::ProtectionUnavailable);
                }
                scheduler.context_cpu_ptr(selection.incoming)
            }
            None => scheduler.active_cpu_ptr(),
        }
    });
    match result {
        Ok(Ok(pointer)) => pointer,
        Ok(Err(_)) | Err(_) => crate::security::launch::recover(),
    }
}

/// Privileged PendSV wrapper for scheduler-owned save/select/restore.
#[cfg(all(feature = "abi-context-switch", target_arch = "arm"))]
#[unsafe(naked)]
#[unsafe(export_name = "PendSV")]
pub(crate) unsafe extern "C" fn pendsv_handler() -> ! {
    core::arch::naked_asm!(
        "stmdb sp!, {{r4-r11}}",
        "mrs r1, psp",
        "mrs r2, control",
        "mov r3, lr",
        "mov r0, sp",
        "bl {prepare}",
        "add sp, #32",
        "b {restore}",
        prepare = sym prepare_pendsv,
        restore = sym crate::runtime::scheduling::context_switch::restore_selected,
    );
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
        crate::platform::request_context_switch();
    }
}

#[cfg(feature = "abi-context-switch")]
/// Dispatches the platform scheduler tick interrupt.
#[unsafe(export_name = "SysTick")]
extern "C" fn systick_handler() {
    on_systick();
}
