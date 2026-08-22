use super::*;
use crate::{
    BoundedText, KeyId, MetadataHeader, MetadataRole, PublicKey, RevocationMetadata,
    RevocationRecord, RoleDefinition, RoleKey, RootMetadata,
};

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

#[test]
fn preserves_rotated_root_keys_and_revocations() {
    let key_a = RoleKey {
        role: MetadataRole::Root,
        key_id: KeyId([1; crate::KEY_ID_LENGTH]),
        public_key: PublicKey([2; crate::PUBLIC_KEY_LENGTH]),
    };
    let key_b = RoleKey {
        role: MetadataRole::Root,
        key_id: KeyId([3; crate::KEY_ID_LENGTH]),
        public_key: PublicKey([4; crate::PUBLIC_KEY_LENGTH]),
    };
    let mut keys = [key_a; crate::MAX_ROOT_KEYS];
    keys[1] = key_b;
    let root = RootMetadata {
        header: MetadataHeader {
            role: MetadataRole::Root,
            version: 2,
            expires: 0,
        },
        keys,
        key_count: 2,
        roles: [RoleDefinition {
            role: MetadataRole::Root,
            keys: [key_a.key_id; crate::MAX_ROLE_KEYS],
            key_count: 1,
            threshold: 1,
        }; crate::MAX_ROOT_ROLES],
        role_count: 1,
    };
    let mut records = [RevocationRecord::default(); crate::MAX_REVOCATIONS];
    records[0] = RevocationRecord {
        developer_id: BoundedText::new("developer").expect("developer fits"),
        effective_version: 2,
        issuer_key_id: key_a.key_id,
        key_id: KeyId([5; crate::KEY_ID_LENGTH]),
        reason: BoundedText::new("rotated").expect("reason fits"),
    };
    let revocations = RevocationMetadata {
        header: MetadataHeader {
            role: MetadataRole::Revocation,
            version: 2,
            expires: 0,
        },
        records,
        record_count: 1,
    };
    let state = TrustStoreSecurityState::from_verified_metadata(&root, &revocations)
        .expect("verified security state reconstructs");
    assert!(state.contains_root_key(key_a.key_id));
    assert!(state.contains_root_key(key_b.key_id));
    assert!(state.is_revoked(KeyId([5; crate::KEY_ID_LENGTH])));
}
