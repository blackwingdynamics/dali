//! One-time kernel-owned storage for scheduler state.

use core::{
    cell::UnsafeCell,
    mem::MaybeUninit,
    sync::atomic::{AtomicU8, Ordering},
};

const UNINITIALIZED: u8 = 0;
const INITIALIZING: u8 = 1;
const READY: u8 = 2;

/// Errors returned by scheduler storage ownership operations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SchedulerStorageError {
    /// The storage has already been initialized or is being initialized.
    AlreadyInitialized,
    /// The storage is not ready for access.
    NotInitialized,
}

/// Fixed-address storage initialized once during kernel bootstrap.
pub struct SchedulerStorage<T> {
    state: AtomicU8,
    value: UnsafeCell<MaybeUninit<T>>,
}

// SAFETY: The storage is only accessed through `get_mut`, whose caller must
// provide exclusive interrupt-masked access to the scheduler state.
unsafe impl<T: Send> Sync for SchedulerStorage<T> {}

impl<T> SchedulerStorage<T> {
    /// Creates storage without constructing the scheduler value.
    pub const fn new() -> Self {
        Self {
            state: AtomicU8::new(UNINITIALIZED),
            value: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }

    /// Initializes the storage exactly once.
    pub fn initialize(&self, value: T) -> Result<(), SchedulerStorageError> {
        if self
            .state
            .compare_exchange(
                UNINITIALIZED,
                INITIALIZING,
                Ordering::Acquire,
                Ordering::Acquire,
            )
            .is_err()
        {
            return Err(SchedulerStorageError::AlreadyInitialized);
        }
        // SAFETY: The state transition reserves this storage for the current
        // initializer, and no reader can observe READY before this write.
        unsafe { (*self.value.get()).write(value) };
        self.state.store(READY, Ordering::Release);
        Ok(())
    }

    /// Returns a pointer to the initialized scheduler state for one exclusive
    /// operation.
    ///
    /// # Safety
    ///
    /// The caller must mask scheduler interrupts for the complete operation,
    /// validate the pointer before dereferencing it, and must not create
    /// another mutable access through this storage concurrently.
    pub unsafe fn get_mut_ptr(&self) -> Result<*mut T, SchedulerStorageError> {
        if self.state.load(Ordering::Acquire) != READY {
            return Err(SchedulerStorageError::NotInitialized);
        }
        // SAFETY: The caller guarantees exclusive interrupt-masked access and
        // initialization has published the fully written value.
        Ok(unsafe { (*self.value.get()).as_mut_ptr() })
    }
}

impl<T> Default for SchedulerStorage<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{SchedulerStorage, SchedulerStorageError};

    #[test]
    fn rejects_access_before_initialization() {
        let storage = SchedulerStorage::<u32>::new();
        assert_eq!(
            unsafe { storage.get_mut_ptr() },
            Err(SchedulerStorageError::NotInitialized)
        );
    }

    #[test]
    fn publishes_initialized_value_once() {
        let storage = SchedulerStorage::new();
        assert_eq!(storage.initialize(7), Ok(()));
        assert_eq!(
            storage.initialize(9),
            Err(SchedulerStorageError::AlreadyInitialized)
        );
        assert_eq!(
            unsafe { storage.get_mut_ptr() }.map(|value| unsafe { *value }),
            Ok(7)
        );
    }
}
