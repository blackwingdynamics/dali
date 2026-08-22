//! Recovery selection for the two-record commit journal.

use super::super::{COMMIT_JOURNAL_BYTES, COMMIT_JOURNAL_RECORD_BYTES};
use super::{CommitJournalRecord, CommitJournalState, JournalError, RecoveryDecision};

/// Selects the durable generation from the two fixed journal records.
///
/// Torn or otherwise invalid records are ignored when the other record is
/// valid. Prepared records never select an active slot. Among committed
/// records, the greatest bundle version wins; journal sequence breaks ties.
pub fn recover(input: &[u8]) -> Result<RecoveryDecision, JournalError> {
    if input.len() != COMMIT_JOURNAL_BYTES {
        return Err(JournalError::InvalidLength);
    }
    let first = CommitJournalRecord::decode(&input[..COMMIT_JOURNAL_RECORD_BYTES]).ok();
    let second = CommitJournalRecord::decode(&input[COMMIT_JOURNAL_RECORD_BYTES..]).ok();
    let selected = [first, second]
        .into_iter()
        .flatten()
        .filter(|record| record.state == CommitJournalState::Committed)
        .max_by(|left, right| {
            (left.bundle_version, left.sequence).cmp(&(right.bundle_version, right.sequence))
        });
    if let Some(record) = selected {
        return Ok(RecoveryDecision::Committed(record));
    }
    if first
        .into_iter()
        .chain(second)
        .any(|record| record.state == CommitJournalState::Prepared)
    {
        return Ok(RecoveryDecision::DiscardPrepared);
    }
    Err(JournalError::NoCommittedRecord)
}
