//! Disposable acceptance fixture for reboot recovery from Prepared state.

use crate::{
    drivers::{
        BlockReader, BlockTransportFlush, BlockWriter, FlushableBlockDevice, StorageError,
        WritableBlockDeviceAdapter,
    },
    storage::durable::{
        COMMIT_JOURNAL_BYTES, COMMIT_JOURNAL_RECORD_BYTES, CommitJournalState,
        journal::{CommitJournalRecord, JournalSlot},
    },
    storage::filesystem,
};

const PREPARED_SEQUENCE_A: u64 = 7;
const PREPARED_SEQUENCE_B: u64 = 8;
const PREPARED_VERSION: u64 = 2;
const PREPARED_LENGTH: u32 = 32;
const PREPARED_DIGEST: [u8; 32] = [0xD7; 32];

/// Stages a durable journal containing no committed record.
pub(super) fn stage_prepared_journal<R>(
    device: &WritableBlockDeviceAdapter<R>,
) -> Result<(), super::acceptance::ArtifactTestError>
where
    R: BlockReader + BlockWriter + BlockTransportFlush<Error = StorageError>,
{
    let prepared_a = prepared_record(PREPARED_SEQUENCE_A);
    let prepared_b = prepared_record(PREPARED_SEQUENCE_B);
    let mut expected = [0; COMMIT_JOURNAL_BYTES];
    prepared_a
        .encode(&mut expected[..COMMIT_JOURNAL_RECORD_BYTES])
        .map_err(super::acceptance::ArtifactTestError::CommitJournal)?;
    prepared_b
        .encode(&mut expected[COMMIT_JOURNAL_RECORD_BYTES..])
        .map_err(super::acceptance::ArtifactTestError::CommitJournal)?;

    filesystem::write_trust_store_artifact(
        device,
        filesystem::TrustStoreArtifact::CommitMarker,
        &expected,
    )
    .map_err(super::acceptance::ArtifactTestError::CommitWrite)?;
    device
        .flush()
        .map_err(embedded_sdmmc::Error::DeviceError)
        .map_err(super::acceptance::ArtifactTestError::CommitFlush)?;

    let mut actual = [0; COMMIT_JOURNAL_BYTES];
    let length = filesystem::read_trust_store_artifact(
        device,
        filesystem::TrustStoreArtifact::CommitMarker,
        &mut actual,
    )
    .map_err(super::acceptance::ArtifactTestError::CommitRead)?;
    if length != expected.len() || actual != expected {
        return Err(super::acceptance::ArtifactTestError::ReadBackMismatch(
            filesystem::TrustStoreArtifact::CommitMarker,
        ));
    }
    Ok(())
}

fn prepared_record(sequence: u64) -> CommitJournalRecord {
    CommitJournalRecord {
        state: CommitJournalState::Prepared,
        active_slot: JournalSlot::B,
        sequence,
        bundle_version: PREPARED_VERSION,
        bundle_length: PREPARED_LENGTH,
        bundle_digest: PREPARED_DIGEST,
    }
}
