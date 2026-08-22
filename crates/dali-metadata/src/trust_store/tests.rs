use super::*;

fn record(version: u64, byte: u8) -> TrustStoreRecord {
    TrustStoreRecord::new(version, Sha256Digest([byte; crate::SHA256_LENGTH]))
        .expect("test record is valid")
}

#[test]
fn commits_a_verified_candidate_and_retains_the_previous_generation() {
    let mut state = TrustStoreState::new(record(1, 1), record(0x10, 0x10));
    let candidate = record(2, 2);
    state.stage(candidate).expect("candidate stages");
    state.mark_verified(candidate).expect("candidate verifies");
    state.begin_commit().expect("commit marker starts");
    state.finalize_commit().expect("commit finalizes");
    assert_eq!(state.active(), candidate);
    assert_eq!(state.fallback(), record(1, 1));
    assert_eq!(state.phase(), TrustStorePhase::Active);
}

#[test]
fn discards_an_uncommitted_candidate_after_power_loss() {
    let mut state = TrustStoreState::new(record(1, 1), record(0x10, 0x10));
    state.stage(record(2, 2)).expect("candidate stages");
    state.recover().expect("recovery succeeds");
    assert_eq!(state.active(), record(1, 1));
    assert_eq!(state.candidate(), None);
}

#[test]
fn completes_a_commit_after_power_loss_at_the_commit_marker() {
    let mut state = TrustStoreState::new(record(1, 1), record(0x10, 0x10));
    let candidate = record(2, 2);
    state.stage(candidate).expect("candidate stages");
    state.mark_verified(candidate).expect("candidate verifies");
    state.begin_commit().expect("commit marker starts");
    state.recover().expect("recovery completes commit");
    assert_eq!(state.active(), candidate);
    assert_eq!(state.fallback(), record(1, 1));
}

#[test]
fn rejects_rollback_and_invalid_transitions() {
    let mut state = TrustStoreState::new(record(2, 2), record(1, 1));
    assert_eq!(state.stage(record(2, 3)), Err(TrustStoreError::Rollback));
    assert_eq!(
        state.begin_commit(),
        Err(TrustStoreError::InvalidTransition)
    );
}

#[test]
fn compares_generations_by_monotonic_version() {
    assert!(record(2, 2).is_newer_than(record(1, 1)));
    assert!(!record(1, 1).is_newer_than(record(2, 2)));
}
