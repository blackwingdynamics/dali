//! Generic durable repository persistence state machine.

use super::{
    COMMIT_JOURNAL_BYTES, COMMIT_JOURNAL_RECORD_BYTES, CommitJournalState, DurableArtifact,
    DurableStorageAdapter,
    journal::{CommitJournalRecord, JournalError, JournalSlot},
};

#[cfg(test)]
mod tests;

/// Defines the `SHA256_LENGTH` bound used by this subsystem.
const SHA256_LENGTH: usize = 32;

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

impl DurableGeneration {
    /// Performs the `is_valid` operation for this subsystem.
    pub(crate) fn is_valid(self) -> bool {
        self.version != 0 && self.length != 0 && self.digest != [0; SHA256_LENGTH]
    }

    /// Returns whether this generation advances the active durable generation.
    pub const fn is_newer_than(self, active: Self) -> bool {
        self.version > active.version
    }
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
    /// The candidate generation has no usable durable identity.
    InvalidGeneration,
    /// The verified candidate did not match the staged candidate.
    CandidateMismatch,
}

/// Coordinates candidate publication without assuming a filesystem or board.
pub struct PersistenceCoordinator<S> {
    /// Stores the `storage` value for this bounded state.
    storage: S,
    /// Stores the `state` value for this bounded state.
    state: PersistenceState,
    /// Stores the `active_slot` value for this bounded state.
    active_slot: JournalSlot,
    /// Stores the `active_generation` value for this bounded state.
    active_generation: DurableGeneration,
    /// Stores the `candidate` value for this bounded state.
    candidate: Option<DurableGeneration>,
    /// Stores the `next_sequence` value for this bounded state.
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

    /// Checks candidate identity and monotonicity before any storage write.
    pub fn authorize_candidate(
        &self,
        generation: DurableGeneration,
    ) -> Result<(), PersistenceError<S::Error>> {
        if self.state != PersistenceState::Active {
            return Err(PersistenceError::InvalidTransition);
        }
        if !generation.is_valid() {
            return Err(PersistenceError::InvalidGeneration);
        }
        if !generation.is_newer_than(self.active_generation) {
            return Err(PersistenceError::Rollback);
        }
        Ok(())
    }

    /// Writes candidate bytes and makes them durable.
    pub fn write_candidate(
        &mut self,
        contents: &[u8],
        generation: DurableGeneration,
    ) -> Result<(), PersistenceError<S::Error>> {
        self.authorize_candidate(generation)?;
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

    /// Performs the `record` operation for this subsystem.
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

    /// Performs the `inactive_slot` operation for this subsystem.
    fn inactive_slot(&self) -> JournalSlot {
        match self.active_slot {
            JournalSlot::A => JournalSlot::B,
            JournalSlot::B => JournalSlot::A,
        }
    }

    /// Performs the `inactive_artifact` operation for this subsystem.
    fn inactive_artifact(&self) -> DurableArtifact {
        match self.inactive_slot() {
            JournalSlot::A => DurableArtifact::SlotA,
            JournalSlot::B => DurableArtifact::SlotB,
        }
    }

    /// Performs the `write_journal_pair` operation for this subsystem.
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
