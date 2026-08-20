//! Fixed-capacity metadata contract records.

use crate::{KEY_ID_LENGTH, PUBLIC_KEY_LENGTH, SIGNATURE_LENGTH};

/// The role of one signed metadata document.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MetadataRole {
    /// Root keys and role thresholds.
    Root,
    /// Short-lived reference to one snapshot.
    Timestamp,
    /// Consistent references to targets and delegations.
    Snapshot,
    /// Package records and delegation identifiers.
    Targets,
    /// One developer public key and bounded permissions.
    Delegation,
}

impl MetadataRole {
    /// Returns the canonical wire name for this role.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Root => "root",
            Self::Timestamp => "timestamp",
            Self::Snapshot => "snapshot",
            Self::Targets => "targets",
            Self::Delegation => "delegation",
        }
    }
}

/// Fixed-size opaque key identifier.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KeyId(pub [u8; KEY_ID_LENGTH]);

/// Fixed-size Ed25519 public key.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PublicKey(pub [u8; PUBLIC_KEY_LENGTH]);

/// Fixed-size Ed25519 signature.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Signature(pub [u8; SIGNATURE_LENGTH]);

/// Common fields shared by every signed metadata role.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MetadataHeader {
    /// The role-specific metadata role.
    pub role: MetadataRole,
    /// Monotonic version for this role.
    pub version: u64,
    /// Unix expiration time; zero means no trusted wall clock is available.
    pub expires: u64,
}

/// One public key authorized for a repository role.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RoleKey {
    /// Repository role authorized to use this key.
    pub role: MetadataRole,
    /// Key identifier selected by a signature.
    pub key_id: KeyId,
    /// Ed25519 public key bytes.
    pub public_key: PublicKey,
}

/// One repository role and its signature threshold.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RoleDefinition {
    /// Canonical role name.
    pub role: MetadataRole,
    /// Authorized keys; only the first `key_count` entries are active.
    pub keys: [KeyId; crate::MAX_ROLE_KEYS],
    /// Number of active entries in `keys`.
    pub key_count: u8,
    /// Number of distinct valid signatures required.
    pub threshold: u8,
}

/// Decoded root metadata held in fixed-capacity storage.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RootMetadata {
    /// Common signed metadata fields.
    pub header: MetadataHeader,
    /// Root and delegated public keys.
    pub keys: [RoleKey; crate::MAX_ROOT_KEYS],
    /// Number of active entries in `keys`.
    pub key_count: u8,
    /// Repository role definitions.
    pub roles: [RoleDefinition; crate::MAX_ROOT_ROLES],
    /// Number of active entries in `roles`.
    pub role_count: u8,
}
