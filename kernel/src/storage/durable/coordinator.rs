//! Generic durable repository persistence state machine.

use super::{
    COMMIT_JOURNAL_BYTES, COMMIT_JOURNAL_RECORD_BYTES, CommitJournalState, DurableArtifact,
    DurableStorageAdapter,
    journal::{CommitJournalRecord, JournalError, JournalSlot},
};

/// A verified repository generation independent of its wire encoding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DurableGeneration {
    /// Monotonic repository generation.
    pub version: u64,
    /// SHA-256 digest of the complete repository bundle.
    pub digest: [u8; 32],
    /// Exact bundle length.
    pub length: u32,
}

/// Observable coordinator state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PersistenceState {
    /// The active slot is trusted and selected.
    Active,
    /// Candidate bytes have been durably written.
    CandidateWritten,
    /// Candidate authenticity and policy checks have passed.
    CandidateVerified,
    /// The prepared commit marker is durable.
    CommitPending,
}

/// Errors preserving the phase at which persistence failed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PersistenceError<E> {
    /// The storage adapter failed.
    Storage(E),
    /// The journal record could not be encoded.
    Journal(JournalError),
    /// A method was called outside its allowed phase.
    InvalidTransition,
    /// The candidate generation is not newer than the committed generation.
    Rollback,
    /// The verified candidate did not match the staged candidate.
    CandidateMismatch,
}

/// Coordinates candidate publication without assuming a filesystem or board.
pub struct PersistenceCoordinator<S> {
    storage: S,
    state: PersistenceState,
    active_slot: JournalSlot,
    active_generation: DurableGeneration,
    candidate: Option<DurableGeneration>,
    next_sequence: u64,
}

impl<S> PersistenceCoordinator<S>
where
    S: DurableStorageAdapter,
{
    /// Creates a coordinator from the currently committed generation.
    pub const fn new(
        storage: S,
        active_slot: JournalSlot,
        active_generation: DurableGeneration,
        next_sequence: u64,
    ) -> Self {
        Self {
            storage,
            state: PersistenceState::Active,
            active_slot,
            active_generation,
            candidate: None,
            next_sequence,
        }
    }

    /// Returns the current state without mutating storage.
    pub const fn state(&self) -> PersistenceState {
        self.state
    }

    /// Writes candidate bytes and makes them durable.
    pub fn write_candidate(
        &mut self,
        contents: &[u8],
        generation: DurableGeneration,
    ) -> Result<(), PersistenceError<S::Error>> {
        if self.state != PersistenceState::Active {
            return Err(PersistenceError::InvalidTransition);
        }
        if generation.version <= self.active_generation.version {
            return Err(PersistenceError::Rollback);
        }
        if contents.len() != generation.length as usize {
            return Err(PersistenceError::InvalidTransition);
        }
        self.storage
            .write_artifact(self.inactive_artifact(), contents)
            .map_err(PersistenceError::Storage)?;
        self.storage.flush().map_err(PersistenceError::Storage)?;
        self.candidate = Some(generation);
        self.state = PersistenceState::CandidateWritten;
        Ok(())
    }

    /// Reads the staged slot back and records that external verification succeeded.
    pub fn verify_candidate(
        &mut self,
        generation: DurableGeneration,
        expected_contents: &[u8],
        readback: &mut [u8],
    ) -> Result<(), PersistenceError<S::Error>> {
        if self.state != PersistenceState::CandidateWritten
            || self.candidate != Some(generation)
            || expected_contents.len() != generation.length as usize
            || readback.len() < expected_contents.len()
        {
            return Err(PersistenceError::CandidateMismatch);
        }
        let length = self
            .storage
            .read_artifact(self.inactive_artifact(), readback)
            .map_err(PersistenceError::Storage)?;
        if length != expected_contents.len() || readback[..length] != *expected_contents {
            return Err(PersistenceError::CandidateMismatch);
        }
        self.state = PersistenceState::CandidateVerified;
        Ok(())
    }

    /// Publishes the verified candidate through prepared then committed records.
    pub fn commit(&mut self) -> Result<(), PersistenceError<S::Error>> {
        if self.state != PersistenceState::CandidateVerified {
            return Err(PersistenceError::InvalidTransition);
        }
        let generation = self.candidate.ok_or(PersistenceError::CandidateMismatch)?;
        let inactive_slot = self.inactive_slot();
        let prepared = self.record(CommitJournalState::Prepared, inactive_slot, generation);
        self.write_journal_pair(&prepared, &prepared)?;
        self.state = PersistenceState::CommitPending;
        let committed = self.record(CommitJournalState::Committed, inactive_slot, generation);
        self.write_journal_pair(&committed, &committed)?;
        self.active_slot = inactive_slot;
        self.active_generation = generation;
        self.candidate = None;
        self.state = PersistenceState::Active;
        Ok(())
    }

    /// Returns the owned storage after the coordinator reaches a stable state.
    pub fn into_storage(self) -> S {
        self.storage
    }

    fn record(
        &mut self,
        state: CommitJournalState,
        active_slot: JournalSlot,
        generation: DurableGeneration,
    ) -> CommitJournalRecord {
        let record = CommitJournalRecord {
            state,
            active_slot,
            sequence: self.next_sequence,
            bundle_version: generation.version,
            bundle_length: generation.length,
            bundle_digest: generation.digest,
        };
        self.next_sequence = self.next_sequence.saturating_add(1);
        record
    }

    fn inactive_slot(&self) -> JournalSlot {
        match self.active_slot {
            JournalSlot::A => JournalSlot::B,
            JournalSlot::B => JournalSlot::A,
        }
    }

    fn inactive_artifact(&self) -> DurableArtifact {
        match self.inactive_slot() {
            JournalSlot::A => DurableArtifact::SlotA,
            JournalSlot::B => DurableArtifact::SlotB,
        }
    }

    fn write_journal_pair(
        &mut self,
        first: &CommitJournalRecord,
        second: &CommitJournalRecord,
    ) -> Result<(), PersistenceError<S::Error>> {
        let mut journal = [0; COMMIT_JOURNAL_BYTES];
        first
            .encode(&mut journal[..COMMIT_JOURNAL_RECORD_BYTES])
            .map_err(PersistenceError::Journal)?;
        second
            .encode(&mut journal[COMMIT_JOURNAL_RECORD_BYTES..])
            .map_err(PersistenceError::Journal)?;
        self.storage
            .write_artifact(DurableArtifact::CommitJournal, &journal)
            .map_err(PersistenceError::Storage)?;
        self.storage.flush().map_err(PersistenceError::Storage)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::durable::DurableArtifact;

    struct TestStorage {
        writes: usize,
        candidate: [u8; 4],
    }

    impl DurableStorageAdapter for TestStorage {
        type Error = ();

        fn read_artifact(
            &mut self,
            artifact: DurableArtifact,
            output: &mut [u8],
        ) -> Result<usize, Self::Error> {
            if artifact == DurableArtifact::SlotB {
                output[..self.candidate.len()].copy_from_slice(&self.candidate);
                Ok(self.candidate.len())
            } else {
                Ok(0)
            }
        }

        fn write_artifact(&mut self, _: DurableArtifact, _: &[u8]) -> Result<(), Self::Error> {
            self.writes += 1;
            Ok(())
        }

        fn flush(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    fn generation(version: u64) -> DurableGeneration {
        DurableGeneration {
            version,
            digest: [0x11; 32],
            length: 4,
        }
    }

    #[test]
    fn advances_only_after_each_durable_transition() {
        let mut coordinator = PersistenceCoordinator::new(
            TestStorage {
                writes: 0,
                candidate: [1, 2, 3, 4],
            },
            JournalSlot::A,
            generation(1),
            2,
        );
        assert_eq!(
            coordinator.write_candidate(&[1, 2, 3, 4], generation(2)),
            Ok(())
        );
        assert_eq!(coordinator.state(), PersistenceState::CandidateWritten);
        let mut readback = [0; 4];
        assert_eq!(
            coordinator.verify_candidate(generation(2), &[1, 2, 3, 4], &mut readback),
            Ok(())
        );
        assert_eq!(coordinator.state(), PersistenceState::CandidateVerified);
        assert_eq!(coordinator.commit(), Ok(()));
        assert_eq!(coordinator.state(), PersistenceState::Active);
        assert_eq!(coordinator.into_storage().writes, 3);
    }

    #[test]
    fn rejects_unverified_commit_and_mismatched_generation() {
        let mut coordinator = PersistenceCoordinator::new(
            TestStorage {
                writes: 0,
                candidate: [1, 2, 3, 4],
            },
            JournalSlot::A,
            generation(1),
            2,
        );
        assert_eq!(
            coordinator.commit(),
            Err(PersistenceError::InvalidTransition)
        );
        coordinator
            .write_candidate(&[1, 2, 3, 4], generation(2))
            .expect("candidate writes");
        assert_eq!(
            coordinator.verify_candidate(generation(3), &[1, 2, 3, 4], &mut [0; 4]),
            Err(PersistenceError::CandidateMismatch)
        );
    }

    #[test]
    fn rejects_equal_and_older_generations_before_writing() {
        let mut coordinator = PersistenceCoordinator::new(
            TestStorage {
                writes: 0,
                candidate: [1, 2, 3, 4],
            },
            JournalSlot::A,
            generation(2),
            3,
        );

        assert_eq!(
            coordinator.write_candidate(&[1, 2, 3, 4], generation(2)),
            Err(PersistenceError::Rollback)
        );
        assert_eq!(
            coordinator.write_candidate(&[1, 2, 3, 4], generation(1)),
            Err(PersistenceError::Rollback)
        );
        assert_eq!(coordinator.into_storage().writes, 0);
    }
}
