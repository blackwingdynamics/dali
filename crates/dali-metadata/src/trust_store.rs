//! Hardware-neutral atomic trust-store update state machine.

use crate::{KeyId, RevocationMetadata, RoleKey, RootMetadata, Sha256Digest};

/// One durable trust-store generation identified by version and digest.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TrustStoreRecord {
    /// Strictly increasing durable trust-store version.
    pub version: u64,
    /// Digest of the complete verified trust-store bundle.
    pub sha256: Sha256Digest,
}

impl TrustStoreRecord {
    /// Creates a record after validating its identity fields.
    pub fn new(version: u64, sha256: Sha256Digest) -> Option<Self> {
        if version == 0 || sha256.0 == [0; crate::SHA256_LENGTH] {
            None
        } else {
            Some(Self { version, sha256 })
        }
    }

    /// Returns whether this record advances the durable generation.
    pub const fn is_newer_than(&self, previous: Self) -> bool {
        self.version > previous.version
    }
}

/// Durable phase of a two-slot trust-store update.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrustStorePhase {
    /// The active slot is authoritative and no update is pending.
    Active,
    /// A candidate exists but has not completed verification.
    CandidateReady,
    /// The candidate passed verification and may be committed.
    Verified,
    /// The commit marker is durable; recovery must finish the swap.
    CommitPending,
}

/// Errors returned by the bounded trust-store state machine.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrustStoreError {
    /// A record has an invalid version or digest.
    InvalidRecord,
    /// The candidate version is not newer than the active generation.
    Rollback,
    /// The requested candidate does not match the staged record.
    CandidateMismatch,
    /// The transition is not valid for the current durable phase.
    InvalidTransition,
    /// Verified root or revocation state exceeds fixed-capacity bounds.
    InvalidSecurityState,
}

/// Verified key and revocation state reconstructed from signed metadata.
///
/// Root key rotation keeps old and new keys concurrently available until the
/// signed root metadata removes the old key. Revocations remain effective
/// because their key identifiers are retained with the committed generation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TrustStoreSecurityState {
    root_keys: [Option<RoleKey>; crate::MAX_ROOT_KEYS],
    root_key_count: u8,
    revoked_keys: [Option<KeyId>; crate::MAX_REVOCATIONS],
    revoked_key_count: u8,
}

impl TrustStoreSecurityState {
    /// Reconstructs security state from an already verified root and revocation document.
    pub fn from_verified_metadata(
        root: &RootMetadata,
        revocations: &RevocationMetadata,
    ) -> Result<Self, TrustStoreError> {
        let root_count = usize::from(root.key_count);
        let revoked_count = usize::from(revocations.record_count);
        if root_count == 0
            || root_count > crate::MAX_ROOT_KEYS
            || revoked_count > crate::MAX_REVOCATIONS
        {
            return Err(TrustStoreError::InvalidSecurityState);
        }
        let mut state = Self {
            root_keys: [None; crate::MAX_ROOT_KEYS],
            root_key_count: root.key_count,
            revoked_keys: [None; crate::MAX_REVOCATIONS],
            revoked_key_count: revocations.record_count,
        };
        for (index, key) in root.keys.iter().take(root_count).enumerate() {
            if key.key_id.0 == [0; crate::KEY_ID_LENGTH]
                || state.root_keys[..index]
                    .iter()
                    .flatten()
                    .any(|candidate| candidate.key_id == key.key_id)
            {
                return Err(TrustStoreError::InvalidSecurityState);
            }
            state.root_keys[index] = Some(*key);
        }
        for (index, record) in revocations.records.iter().take(revoked_count).enumerate() {
            if record.key_id.0 == [0; crate::KEY_ID_LENGTH]
                || state.revoked_keys[..index]
                    .iter()
                    .flatten()
                    .any(|candidate| *candidate == record.key_id)
            {
                return Err(TrustStoreError::InvalidSecurityState);
            }
            state.revoked_keys[index] = Some(record.key_id);
        }
        Ok(state)
    }

    /// Returns whether a key is present in the committed root key set.
    pub fn contains_root_key(&self, key_id: KeyId) -> bool {
        self.root_keys[..usize::from(self.root_key_count)]
            .iter()
            .flatten()
            .any(|key| key.key_id == key_id)
    }

    /// Returns whether a developer key is revoked in the committed state.
    pub fn is_revoked(&self, key_id: KeyId) -> bool {
        self.revoked_keys[..usize::from(self.revoked_key_count)]
            .iter()
            .flatten()
            .any(|revoked| *revoked == key_id)
    }
}

/// Two-slot trust-store state without owning storage or bundle bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TrustStoreState {
    active: TrustStoreRecord,
    fallback: TrustStoreRecord,
    candidate: Option<TrustStoreRecord>,
    phase: TrustStorePhase,
}

impl TrustStoreState {
    /// Creates a stable state with one active generation and its fallback.
    pub const fn new(active: TrustStoreRecord, fallback: TrustStoreRecord) -> Self {
        Self {
            active,
            fallback,
            candidate: None,
            phase: TrustStorePhase::Active,
        }
    }

    /// Returns the currently authoritative generation.
    pub const fn active(&self) -> TrustStoreRecord {
        self.active
    }

    /// Returns the previous valid generation retained for recovery.
    pub const fn fallback(&self) -> TrustStoreRecord {
        self.fallback
    }

    /// Returns the pending candidate, if any.
    pub const fn candidate(&self) -> Option<TrustStoreRecord> {
        self.candidate
    }

    /// Returns the current durable phase.
    pub const fn phase(&self) -> TrustStorePhase {
        self.phase
    }

    /// Stages a strictly newer candidate in the inactive slot.
    pub fn stage(&mut self, candidate: TrustStoreRecord) -> Result<(), TrustStoreError> {
        if candidate.version == 0 || candidate.sha256.0 == [0; crate::SHA256_LENGTH] {
            return Err(TrustStoreError::InvalidRecord);
        }
        if candidate.version <= self.active.version {
            return Err(TrustStoreError::Rollback);
        }
        if self.phase != TrustStorePhase::Active {
            return Err(TrustStoreError::InvalidTransition);
        }
        self.candidate = Some(candidate);
        self.phase = TrustStorePhase::CandidateReady;
        Ok(())
    }

    /// Marks the staged candidate verified after storage read-back.
    pub fn mark_verified(&mut self, candidate: TrustStoreRecord) -> Result<(), TrustStoreError> {
        if self.phase != TrustStorePhase::CandidateReady || self.candidate != Some(candidate) {
            return Err(TrustStoreError::CandidateMismatch);
        }
        self.phase = TrustStorePhase::Verified;
        Ok(())
    }

    /// Records the durable commit marker before swapping the active slot.
    pub fn begin_commit(&mut self) -> Result<(), TrustStoreError> {
        if self.phase != TrustStorePhase::Verified {
            return Err(TrustStoreError::InvalidTransition);
        }
        self.phase = TrustStorePhase::CommitPending;
        Ok(())
    }

    /// Finalizes the active/candidate swap after the commit marker is durable.
    pub fn finalize_commit(&mut self) -> Result<(), TrustStoreError> {
        if self.phase != TrustStorePhase::CommitPending {
            return Err(TrustStoreError::InvalidTransition);
        }
        let candidate = self.candidate.ok_or(TrustStoreError::CandidateMismatch)?;
        self.fallback = self.active;
        self.active = candidate;
        self.candidate = None;
        self.phase = TrustStorePhase::Active;
        Ok(())
    }

    /// Recovers after power loss at any persisted transition boundary.
    pub fn recover(&mut self) -> Result<(), TrustStoreError> {
        match self.phase {
            TrustStorePhase::Active => Ok(()),
            TrustStorePhase::CandidateReady | TrustStorePhase::Verified => {
                self.candidate = None;
                self.phase = TrustStorePhase::Active;
                Ok(())
            }
            TrustStorePhase::CommitPending => self.finalize_commit(),
        }
    }
}

#[cfg(test)]
mod tests;
