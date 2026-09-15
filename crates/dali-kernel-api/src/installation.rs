//! Hardware-neutral contracts for staged artifact installation.

use crate::storage::StorageError;

/// The lifecycle of a single physical artifact slot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SingleSlotReplacementState {
    /// The slot does not contain a bootable artifact.
    Empty,
    /// A replacement is being written and the previous artifact is no longer
    /// protected from replacement.
    Receiving {
        /// Expected complete artifact length.
        expected_length: u32,
        /// Number of sequential bytes accepted so far.
        received_length: u32,
    },
    /// The complete replacement passed the normal AMRN validation boundary.
    Validated {
        /// Complete validated artifact length.
        length: u32,
    },
    /// The replacement is published and may be selected for boot.
    Published {
        /// Complete published artifact length.
        length: u32,
    },
}

/// Errors from the hardware-neutral single-slot replacement policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SingleSlotReplacementError {
    /// The requested artifact length is outside the configured capacity.
    InvalidLength,
    /// The operation is not valid for the current lifecycle state.
    InvalidState,
    /// The write would create a gap, overlap, or exceed the candidate length.
    InvalidWriteRange,
    /// The candidate is not complete yet.
    Incomplete,
}

/// Tracks the safety policy for replacing an artifact in one physical slot.
///
/// This policy intentionally does not provide persistence or hardware
/// operations. A board adapter must store an explicit publication marker and
/// reconstruct `Empty` after an interrupted replacement.
pub struct SingleSlotReplacement {
    capacity: u32,
    state: SingleSlotReplacementState,
}

impl SingleSlotReplacement {
    /// Creates an empty policy state with a bounded artifact capacity.
    pub const fn new(capacity: u32) -> Self {
        Self {
            capacity,
            state: SingleSlotReplacementState::Empty,
        }
    }

    /// Returns the configured logical artifact capacity.
    pub const fn capacity(&self) -> u32 {
        self.capacity
    }

    /// Returns the current lifecycle state.
    pub const fn state(&self) -> SingleSlotReplacementState {
        self.state
    }

    /// Returns the candidate length after complete validation.
    pub const fn validated_length(&self) -> Option<u32> {
        match self.state {
            SingleSlotReplacementState::Validated { length }
            | SingleSlotReplacementState::Published { length } => Some(length),
            _ => None,
        }
    }

    /// Invalidates the current slot and starts a sequential replacement.
    pub fn begin(&mut self, length: u32) -> Result<(), SingleSlotReplacementError> {
        if length == 0 || length > self.capacity {
            return Err(SingleSlotReplacementError::InvalidLength);
        }
        self.state = SingleSlotReplacementState::Receiving {
            expected_length: length,
            received_length: 0,
        };
        Ok(())
    }

    /// Records one contiguous write accepted by the board adapter.
    pub fn record_write(
        &mut self,
        offset: u32,
        length: u32,
    ) -> Result<(), SingleSlotReplacementError> {
        let SingleSlotReplacementState::Receiving {
            expected_length,
            received_length,
        } = self.state
        else {
            return Err(SingleSlotReplacementError::InvalidState);
        };
        let end = offset
            .checked_add(length)
            .ok_or(SingleSlotReplacementError::InvalidWriteRange)?;
        if offset != received_length || length == 0 || end > expected_length {
            return Err(SingleSlotReplacementError::InvalidWriteRange);
        }
        self.state = SingleSlotReplacementState::Receiving {
            expected_length,
            received_length: end,
        };
        Ok(())
    }

    /// Records that the complete candidate passed AMRN validation.
    pub fn mark_validated(&mut self) -> Result<(), SingleSlotReplacementError> {
        let SingleSlotReplacementState::Receiving {
            expected_length,
            received_length,
        } = self.state
        else {
            return Err(SingleSlotReplacementError::InvalidState);
        };
        if received_length != expected_length {
            return Err(SingleSlotReplacementError::Incomplete);
        }
        self.state = SingleSlotReplacementState::Validated {
            length: expected_length,
        };
        Ok(())
    }

    /// Publishes the validated candidate as the bootable artifact.
    pub fn publish(&mut self) -> Result<(), SingleSlotReplacementError> {
        let SingleSlotReplacementState::Validated { length } = self.state else {
            return Err(SingleSlotReplacementError::InvalidState);
        };
        self.state = SingleSlotReplacementState::Published { length };
        Ok(())
    }

    /// Reconstructs the non-bootable state after interruption or cancellation.
    pub fn recover_empty(&mut self) {
        self.state = SingleSlotReplacementState::Empty;
    }
}

/// A persistent writer for an uncommitted artifact staging area.
pub trait ArtifactStager {
    /// Returns the maximum artifact length accepted by the staging area.
    fn staging_capacity(&self) -> Result<u32, StorageError>;

    /// Invalidates any previous candidate and starts a bounded new transfer.
    ///
    /// The candidate must not become visible to the boot source until
    /// [`Self::commit_staging`] succeeds.
    fn begin_staging(&mut self, length: u32) -> Result<(), StorageError>;

    /// Writes one complete caller-owned chunk into the candidate artifact.
    ///
    /// Implementations must reject ranges outside the candidate length and
    /// must not publish a partially written candidate as active.
    fn write_staging(&mut self, offset: u32, bytes: &[u8]) -> Result<(), StorageError>;

    /// Publishes the completely validated candidate as the active artifact.
    ///
    /// The caller must validate the staged bytes through the normal AMRN
    /// loader contract before invoking this operation.
    fn commit_staging(&mut self) -> Result<(), StorageError>;

    /// Discards the current candidate without changing the active artifact.
    fn abort_staging(&mut self) -> Result<(), StorageError>;
}

#[cfg(test)]
mod tests {
    use super::{SingleSlotReplacement, SingleSlotReplacementError, SingleSlotReplacementState};

    #[test]
    fn publishes_only_after_complete_validation() {
        let mut replacement = SingleSlotReplacement::new(64);
        replacement.begin(8).unwrap();
        replacement.record_write(0, 4).unwrap();
        assert_eq!(
            replacement.mark_validated(),
            Err(SingleSlotReplacementError::Incomplete)
        );
        replacement.record_write(4, 4).unwrap();
        replacement.mark_validated().unwrap();
        replacement.publish().unwrap();
        assert_eq!(
            replacement.state(),
            SingleSlotReplacementState::Published { length: 8 }
        );
    }

    #[test]
    fn rejects_gaps_and_overlapping_writes() {
        let mut replacement = SingleSlotReplacement::new(64);
        replacement.begin(8).unwrap();
        assert_eq!(
            replacement.record_write(1, 2),
            Err(SingleSlotReplacementError::InvalidWriteRange)
        );
        replacement.record_write(0, 4).unwrap();
        assert_eq!(
            replacement.record_write(2, 2),
            Err(SingleSlotReplacementError::InvalidWriteRange)
        );
    }

    #[test]
    fn interruption_leaves_empty_state() {
        let mut replacement = SingleSlotReplacement::new(64);
        replacement.begin(8).unwrap();
        replacement.record_write(0, 8).unwrap();
        replacement.recover_empty();
        assert_eq!(replacement.state(), SingleSlotReplacementState::Empty);
    }
}
