//! Feature-gated hardware acceptance checks for durable storage artifacts.

use crate::{
    drivers::{
        BlockReader, BlockTransportFlush, BlockWriter, FlushableBlockDevice, StorageError,
        WritableBlockDeviceAdapter,
    },
    storage::{
        durable::{
            COMMIT_JOURNAL_BYTES, COMMIT_JOURNAL_RECORD_BYTES, CommitJournalState,
            RecoveryDecision,
            journal::{CommitJournalRecord, JournalSlot},
            recover_commit_journal,
        },
        filesystem,
    },
};

const ARTIFACT_TEST_LENGTH: usize = 32;
const ACTIVE_TEST_BYTE: u8 = 0xA5;
const CANDIDATE_TEST_BYTE: u8 = 0x5A;
const TEST_JOURNAL_SEQUENCE: u64 = 2;
const TEST_JOURNAL_VERSION: u64 = 1;
const TEST_JOURNAL_LENGTH: u32 = ARTIFACT_TEST_LENGTH as u32;
const TEST_JOURNAL_DIGEST: [u8; 32] = [0xC3; 32];

/// Result of inspecting the commit journal during boot acceptance.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JournalRecovery {
    /// No commit journal exists on the card yet.
    Missing,
    /// The journal was decoded and a recovery decision was produced.
    Decision(RecoveryDecision),
}

/// Identifies the durable artifact operation that failed during acceptance.
pub enum ArtifactTestError {
    /// The active trust-store artifact could not be written.
    ActiveWrite(embedded_sdmmc::Error<StorageError>),
    /// The active trust-store artifact could not be read back.
    ActiveRead(embedded_sdmmc::Error<StorageError>),
    /// The active trust-store artifact could not be flushed.
    ActiveFlush(embedded_sdmmc::Error<StorageError>),
    /// The candidate trust-store artifact could not be written.
    CandidateWrite(embedded_sdmmc::Error<StorageError>),
    /// The candidate trust-store artifact could not be read back.
    CandidateRead(embedded_sdmmc::Error<StorageError>),
    /// The candidate trust-store artifact could not be flushed.
    CandidateFlush(embedded_sdmmc::Error<StorageError>),
    /// The commit marker could not be written.
    CommitWrite(embedded_sdmmc::Error<StorageError>),
    /// The commit marker could not be read back.
    CommitRead(embedded_sdmmc::Error<StorageError>),
    /// The commit marker could not be flushed.
    CommitFlush(embedded_sdmmc::Error<StorageError>),
    /// The commit journal could not be decoded.
    CommitJournal(crate::storage::durable::JournalError),
    /// Read-back data did not match the written bytes.
    ReadBackMismatch(filesystem::TrustStoreArtifact),
}

impl core::fmt::Debug for ArtifactTestError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ActiveWrite(error) => formatter.debug_tuple("ActiveWrite").field(error).finish(),
            Self::ActiveRead(error) => formatter.debug_tuple("ActiveRead").field(error).finish(),
            Self::ActiveFlush(error) => formatter.debug_tuple("ActiveFlush").field(error).finish(),
            Self::CandidateWrite(error) => formatter
                .debug_tuple("CandidateWrite")
                .field(error)
                .finish(),
            Self::CandidateRead(error) => {
                formatter.debug_tuple("CandidateRead").field(error).finish()
            }
            Self::CandidateFlush(error) => formatter
                .debug_tuple("CandidateFlush")
                .field(error)
                .finish(),
            Self::CommitWrite(error) => formatter.debug_tuple("CommitWrite").field(error).finish(),
            Self::CommitRead(error) => formatter.debug_tuple("CommitRead").field(error).finish(),
            Self::CommitFlush(error) => formatter.debug_tuple("CommitFlush").field(error).finish(),
            Self::CommitJournal(error) => {
                formatter.debug_tuple("CommitJournal").field(error).finish()
            }
            Self::ReadBackMismatch(artifact) => formatter
                .debug_tuple("ReadBackMismatch")
                .field(artifact)
                .finish(),
        }
    }
}

/// Verifies persistence through the real filesystem boundary.
///
/// This acceptance-only path overwrites the three reserved artifact names and
/// must therefore be enabled only on a test card.
pub fn verify_trust_store_artifacts<R>(
    device: &WritableBlockDeviceAdapter<R>,
) -> Result<(), ArtifactTestError>
where
    R: BlockReader + BlockWriter + BlockTransportFlush<Error = StorageError>,
{
    let active = [ACTIVE_TEST_BYTE; ARTIFACT_TEST_LENGTH];
    let candidate = [CANDIDATE_TEST_BYTE; ARTIFACT_TEST_LENGTH];

    verify_artifact(device, filesystem::TrustStoreArtifact::Active, &active)?;
    verify_artifact(
        device,
        filesystem::TrustStoreArtifact::Candidate,
        &candidate,
    )?;
    verify_commit_journal(device)
}

/// Reads and selects the durable commit journal during boot acceptance.
pub fn recover_trust_store_journal<R>(
    device: &WritableBlockDeviceAdapter<R>,
) -> Result<JournalRecovery, ArtifactTestError>
where
    R: BlockReader + BlockWriter,
{
    let mut journal = [0; COMMIT_JOURNAL_BYTES];
    let length = match filesystem::read_trust_store_artifact(
        device,
        filesystem::TrustStoreArtifact::CommitMarker,
        &mut journal,
    ) {
        Ok(length) => length,
        Err(embedded_sdmmc::Error::NotFound) => return Ok(JournalRecovery::Missing),
        Err(error) => return Err(ArtifactTestError::CommitRead(error)),
    };
    if length != journal.len() {
        return Err(ArtifactTestError::CommitJournal(
            crate::storage::durable::JournalError::InvalidLength,
        ));
    }
    recover_commit_journal(&journal)
        .map(JournalRecovery::Decision)
        .map_err(ArtifactTestError::CommitJournal)
}

fn verify_artifact<R>(
    device: &WritableBlockDeviceAdapter<R>,
    artifact: filesystem::TrustStoreArtifact,
    expected: &[u8; ARTIFACT_TEST_LENGTH],
) -> Result<(), ArtifactTestError>
where
    R: BlockReader + BlockWriter + BlockTransportFlush<Error = StorageError>,
{
    if let Err(error) = filesystem::write_trust_store_artifact(device, artifact, expected) {
        return Err(write_error(artifact, error));
    }
    if let Err(error) = device.flush().map_err(embedded_sdmmc::Error::DeviceError) {
        return Err(flush_error(artifact, error));
    }
    let mut actual = [0; ARTIFACT_TEST_LENGTH];
    let length = match filesystem::read_trust_store_artifact(device, artifact, &mut actual) {
        Ok(length) => length,
        Err(error) => return Err(read_error(artifact, error)),
    };
    if length != expected.len() || actual != *expected {
        return Err(ArtifactTestError::ReadBackMismatch(artifact));
    }
    Ok(())
}

fn verify_commit_journal<R>(device: &WritableBlockDeviceAdapter<R>) -> Result<(), ArtifactTestError>
where
    R: BlockReader + BlockWriter + BlockTransportFlush<Error = StorageError>,
{
    let prepared = CommitJournalRecord {
        state: CommitJournalState::Prepared,
        active_slot: JournalSlot::B,
        sequence: TEST_JOURNAL_SEQUENCE - 1,
        bundle_version: TEST_JOURNAL_VERSION,
        bundle_length: TEST_JOURNAL_LENGTH,
        bundle_digest: TEST_JOURNAL_DIGEST,
    };
    let committed = CommitJournalRecord {
        state: CommitJournalState::Committed,
        active_slot: JournalSlot::B,
        sequence: TEST_JOURNAL_SEQUENCE,
        bundle_version: TEST_JOURNAL_VERSION,
        bundle_length: TEST_JOURNAL_LENGTH,
        bundle_digest: TEST_JOURNAL_DIGEST,
    };
    let mut expected = [0; COMMIT_JOURNAL_BYTES];
    prepared
        .encode(&mut expected[..COMMIT_JOURNAL_RECORD_BYTES])
        .map_err(ArtifactTestError::CommitJournal)?;
    committed
        .encode(&mut expected[COMMIT_JOURNAL_RECORD_BYTES..])
        .map_err(ArtifactTestError::CommitJournal)?;
    if let Err(error) = filesystem::write_trust_store_artifact(
        device,
        filesystem::TrustStoreArtifact::CommitMarker,
        &expected,
    ) {
        return Err(ArtifactTestError::CommitWrite(error));
    }
    if let Err(error) = device.flush().map_err(embedded_sdmmc::Error::DeviceError) {
        return Err(ArtifactTestError::CommitFlush(error));
    }
    let mut actual = [0; COMMIT_JOURNAL_BYTES];
    let length = filesystem::read_trust_store_artifact(
        device,
        filesystem::TrustStoreArtifact::CommitMarker,
        &mut actual,
    )
    .map_err(ArtifactTestError::CommitRead)?;
    if length != expected.len() || actual != expected {
        return Err(ArtifactTestError::ReadBackMismatch(
            filesystem::TrustStoreArtifact::CommitMarker,
        ));
    }
    Ok(())
}

fn write_error(
    artifact: filesystem::TrustStoreArtifact,
    error: embedded_sdmmc::Error<StorageError>,
) -> ArtifactTestError {
    match artifact {
        filesystem::TrustStoreArtifact::Active => ArtifactTestError::ActiveWrite(error),
        filesystem::TrustStoreArtifact::Candidate => ArtifactTestError::CandidateWrite(error),
        filesystem::TrustStoreArtifact::CommitMarker => ArtifactTestError::CommitWrite(error),
    }
}

fn read_error(
    artifact: filesystem::TrustStoreArtifact,
    error: embedded_sdmmc::Error<StorageError>,
) -> ArtifactTestError {
    match artifact {
        filesystem::TrustStoreArtifact::Active => ArtifactTestError::ActiveRead(error),
        filesystem::TrustStoreArtifact::Candidate => ArtifactTestError::CandidateRead(error),
        filesystem::TrustStoreArtifact::CommitMarker => ArtifactTestError::CommitRead(error),
    }
}

fn flush_error(
    artifact: filesystem::TrustStoreArtifact,
    error: embedded_sdmmc::Error<StorageError>,
) -> ArtifactTestError {
    match artifact {
        filesystem::TrustStoreArtifact::Active => ArtifactTestError::ActiveFlush(error),
        filesystem::TrustStoreArtifact::Candidate => ArtifactTestError::CandidateFlush(error),
        filesystem::TrustStoreArtifact::CommitMarker => ArtifactTestError::CommitFlush(error),
    }
}
