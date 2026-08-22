use super::super::{COMMIT_JOURNAL_BYTES, COMMIT_JOURNAL_RECORD_BYTES, CommitJournalState};
use super::{
    CommitJournalRecord, JournalError, JournalSlot, MAX_BUNDLE_BYTES, RESERVED_OFFSET,
    RecoveryDecision, recover,
};

fn record(state: CommitJournalState, sequence: u64, bundle_version: u64) -> CommitJournalRecord {
    CommitJournalRecord {
        state,
        active_slot: JournalSlot::B,
        sequence,
        bundle_version,
        bundle_length: 512,
        bundle_digest: [0xA5; 32],
    }
}

#[test]
fn round_trips_frozen_record() {
    let mut bytes = [0; COMMIT_JOURNAL_RECORD_BYTES];
    let expected = record(CommitJournalState::Committed, 7, 3);
    expected.encode(&mut bytes).expect("record encodes");
    assert_eq!(CommitJournalRecord::decode(&bytes), Ok(expected));
}

#[test]
fn rejects_tampered_body_and_reserved_bytes() {
    let mut bytes = [0; COMMIT_JOURNAL_RECORD_BYTES];
    record(CommitJournalState::Committed, 7, 3)
        .encode(&mut bytes)
        .expect("record encodes");
    bytes[0x10] ^= 1;
    assert_eq!(
        CommitJournalRecord::decode(&bytes),
        Err(JournalError::InvalidCrc)
    );
    record(CommitJournalState::Committed, 7, 3)
        .encode(&mut bytes)
        .expect("record encodes");
    bytes[RESERVED_OFFSET] = 1;
    assert_eq!(
        CommitJournalRecord::decode(&bytes),
        Err(JournalError::NonZeroReserved)
    );
}

#[test]
fn rejects_oversized_bundle() {
    let mut bytes = [0; COMMIT_JOURNAL_RECORD_BYTES];
    let mut invalid = record(CommitJournalState::Committed, 7, 3);
    invalid.bundle_length = MAX_BUNDLE_BYTES + 1;
    assert_eq!(
        invalid.encode(&mut bytes),
        Err(JournalError::InvalidBundleLength)
    );
}

#[test]
fn rejects_zero_identity_fields() {
    let mut bytes = [0; COMMIT_JOURNAL_RECORD_BYTES];
    let mut invalid = record(CommitJournalState::Committed, 0, 3);
    assert_eq!(
        invalid.encode(&mut bytes),
        Err(JournalError::InvalidSequence)
    );
    invalid = record(CommitJournalState::Committed, 7, 0);
    assert_eq!(
        invalid.encode(&mut bytes),
        Err(JournalError::InvalidBundleVersion)
    );
    invalid = record(CommitJournalState::Committed, 7, 3);
    invalid.bundle_length = 0;
    assert_eq!(
        invalid.encode(&mut bytes),
        Err(JournalError::InvalidBundleLength)
    );
    invalid.bundle_length = 512;
    invalid.bundle_digest = [0; 32];
    assert_eq!(
        invalid.encode(&mut bytes),
        Err(JournalError::InvalidBundleDigest)
    );
}

#[test]
fn selects_the_newest_committed_generation_and_ignores_prepared() {
    let prepared = record(CommitJournalState::Prepared, 12, 8);
    let committed = record(CommitJournalState::Committed, 11, 7);
    let mut journal = [0; COMMIT_JOURNAL_BYTES];
    prepared
        .encode(&mut journal[..COMMIT_JOURNAL_RECORD_BYTES])
        .expect("prepared record encodes");
    committed
        .encode(&mut journal[COMMIT_JOURNAL_RECORD_BYTES..])
        .expect("committed record encodes");

    assert_eq!(
        recover(&journal),
        Ok(RecoveryDecision::Committed(committed))
    );
}

#[test]
fn prefers_version_then_sequence_when_both_records_are_committed() {
    let older = record(CommitJournalState::Committed, 20, 9);
    let newer = record(CommitJournalState::Committed, 19, 10);
    let mut journal = [0; COMMIT_JOURNAL_BYTES];
    older
        .encode(&mut journal[..COMMIT_JOURNAL_RECORD_BYTES])
        .expect("older record encodes");
    newer
        .encode(&mut journal[COMMIT_JOURNAL_RECORD_BYTES..])
        .expect("newer record encodes");

    assert_eq!(recover(&journal), Ok(RecoveryDecision::Committed(newer)));
}

#[test]
fn uses_sequence_to_break_equal_version_ties() {
    let older = record(CommitJournalState::Committed, 20, 9);
    let newer = record(CommitJournalState::Committed, 21, 9);
    let mut journal = [0; COMMIT_JOURNAL_BYTES];
    older
        .encode(&mut journal[..COMMIT_JOURNAL_RECORD_BYTES])
        .expect("older record encodes");
    newer
        .encode(&mut journal[COMMIT_JOURNAL_RECORD_BYTES..])
        .expect("newer record encodes");

    assert_eq!(recover(&journal), Ok(RecoveryDecision::Committed(newer)));
}

#[test]
fn discards_prepared_state_when_no_committed_record_survives() {
    let prepared = record(CommitJournalState::Prepared, 12, 8);
    let mut journal = [0; COMMIT_JOURNAL_BYTES];
    prepared
        .encode(&mut journal[..COMMIT_JOURNAL_RECORD_BYTES])
        .expect("prepared record encodes");

    assert_eq!(recover(&journal), Ok(RecoveryDecision::DiscardPrepared));
}

#[test]
fn ignores_one_torn_record_when_the_other_is_committed() {
    let committed = record(CommitJournalState::Committed, 11, 7);
    let mut journal = [0; COMMIT_JOURNAL_BYTES];
    journal[..COMMIT_JOURNAL_RECORD_BYTES].fill(0xFF);
    committed
        .encode(&mut journal[COMMIT_JOURNAL_RECORD_BYTES..])
        .expect("committed record encodes");

    assert_eq!(
        recover(&journal),
        Ok(RecoveryDecision::Committed(committed))
    );
}
